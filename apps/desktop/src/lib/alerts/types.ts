export const EXPORT_FIELDS = [
	'Account',
	'Sender',
	'Subject',
	'Excerpt',
	'ExtractedFacts',
	'Attachments',
	'Body'
] as const;
export type ExportField = (typeof EXPORT_FIELDS)[number];
export interface Destination {
	id: string;
	revision: number;
	name: string;
	kind: 'Local' | 'Webhook';
	enabled: boolean;
	fields: ExportField[];
	public_origin: string | null;
	has_credentials: boolean;
}
export interface History {
	event_id: string;
	destination_id: string;
	status: 'Pending' | 'Accepted' | 'Failed' | 'Canceled' | 'Expired' | 'Unavailable';
	attempts: number;
	created: number;
	finished: number | null;
}
export interface WebhookSecret {
	url: string;
	bearer: string | null;
}
export interface AlertsTransport {
	list(): Promise<Destination[]>;
	save(
		destination: Destination,
		secret: WebhookSecret | null,
		consent: boolean
	): Promise<Destination>;
	remove(id: string, revision: number): Promise<void>;
	history(): Promise<History[]>;
	test(id: string, revision: number, consent: boolean): Promise<void>;
	replay(ids: string[], destination: string, revision: number, consent: boolean): Promise<void>;
	available(): Promise<boolean>;
}
export function newDestination(): Destination {
	return {
		id: crypto.randomUUID(),
		revision: 0,
		name: '',
		kind: 'Local',
		enabled: false,
		fields: [],
		public_origin: null,
		has_credentials: false
	};
}
