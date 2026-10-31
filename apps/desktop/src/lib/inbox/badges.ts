import type { ReaderTransport, TriageBadge } from '../reader/types';
type Request = {
	id: string;
	revision: number;
	active: boolean;
	done: (badges: TriageBadge[]) => void;
};

// INVARIANT: Cache only extraction metadata, never message bodies; bound work to visible demand.
export class BadgeLoader {
	private cache = new Map<string, { revision: number; badges: TriageBadge[] }>();
	private queue: Request[] = [];
	private running = 0;
	constructor(private transport: ReaderTransport) {}
	watch(id: string, done: Request['done'], revision = 0) {
		const cached = this.cache.get(id);
		if (cached?.revision === revision) {
			done(cached.badges);
			return () => {};
		}
		const request = { id, done, revision, active: true };
		this.queue.push(request);
		this.pump();
		return () => {
			request.active = false;
			this.queue = this.queue.filter((item) => item.active);
		};
	}
	private pump() {
		while (this.running < 4 && this.queue.length) {
			const request = this.queue.shift()!;
			if (!request.active) continue;
			this.running++;
			void this.transport
				.body(request.id)
				.then((detail) => {
					if (!request.active || detail.id !== request.id) return;
					const badges = detail.triage_badges;
					this.cache.set(request.id, { revision: request.revision, badges });
					if (this.cache.size > 400) this.cache.delete(this.cache.keys().next().value!);
					request.done(badges);
				})
				.catch(() => {
					// INVARIANT: Extraction failure leaves the summary usable and never displays raw IPC errors.
				})
				.finally(() => {
					this.running--;
					this.pump();
				});
		}
	}
}

const brl = new Intl.NumberFormat('pt-BR', {
	style: 'currency',
	currency: 'BRL'
});
export function badgeText(badge: TriageBadge): {
	label: string;
	value: string;
} {
	if ('OtpCode' in badge) return { label: '2FA / OTP', value: badge.OtpCode.code };
	if ('FebrabanBoleto' in badge) {
		const boleto = badge.FebrabanBoleto;
		return {
			label: 'Boleto',
			value: [
				boleto.amount_cents === null ? null : brl.format(boleto.amount_cents / 100),
				boleto.due_date_iso ? `Due ${boleto.due_date_iso}` : null,
				boleto.digitable_line
			]
				.filter(Boolean)
				.join(' · ')
		};
	}
	if ('PixPayload' in badge) return { label: 'Pix', value: badge.PixPayload.payload };
	if ('NfeInvoice' in badge) return { label: 'NF-e', value: badge.NfeInvoice.access_key };
	if ('TrackingCode' in badge)
		return {
			label: 'Tracking',
			value: `${badge.TrackingCode.carrier} · ${badge.TrackingCode.code}`
		};
	return {
		label: 'Unsubscribe',
		value: badge.UnsubscribeAvailable.supports_one_click
			? 'One-click available'
			: 'One-click unavailable'
	};
}
