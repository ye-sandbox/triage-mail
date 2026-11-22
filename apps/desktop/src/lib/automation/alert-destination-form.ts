import { validateWebhookOrigin } from './simulator';
import type { AlertSecret, Destination } from '$lib/alerts/types';

export function buildDestinationSubmission(params: {
	draft: Destination;
	initialKind: Destination['kind'];
	primaryVal: string;
	secondaryVal: string;
	destConsent: boolean;
	messages: {
		validationNameRequired: string;
		validationWebhookInvalid: string;
		validationFieldsRequired: string;
		validationConsentRequired: string;
	};
}):
	| { ok: true; payload: Destination; secret: AlertSecret | null }
	| { ok: false; error: string } {
	const { draft, primaryVal, secondaryVal, destConsent, messages } = params;
	const trimmedName = draft.name.trim();
	if (!trimmedName || trimmedName.length > 120) {
		return { ok: false, error: messages.validationNameRequired };
	}

	let secret: AlertSecret | null = null;
	let publicOrigin = draft.public_origin;

	if (draft.kind === 'Telegram' && (primaryVal || secondaryVal)) {
		secret = { token: primaryVal, chat_id: secondaryVal };
	} else if (draft.kind === 'Discord' && primaryVal) {
		secret = { url: primaryVal, bearer: null };
	} else if (draft.kind === 'Webhook') {
		if (primaryVal || !publicOrigin) {
			const originCheck = validateWebhookOrigin(primaryVal || publicOrigin || '');
			if (originCheck.error) return { ok: false, error: messages.validationWebhookInvalid };
			if (originCheck.origin) publicOrigin = originCheck.origin;
		}
		if (primaryVal) secret = { url: primaryVal, bearer: secondaryVal || null };
	}

	if (draft.kind !== 'Local' && draft.fields.length === 0) {
		return { ok: false, error: messages.validationFieldsRequired };
	}
	if (draft.enabled && !destConsent) {
		return { ok: false, error: messages.validationConsentRequired };
	}

	return {
		ok: true,
		secret,
		payload: {
			id: draft.id,
			revision: draft.revision,
			name: trimmedName,
			kind: draft.kind,
			enabled: draft.enabled,
			fields: draft.kind === 'Local' ? [] : [...draft.fields],
			public_origin: draft.kind === 'Local' ? null : publicOrigin,
			has_credentials: draft.has_credentials
		}
	};
}
