import { invoke } from '@tauri-apps/api/core';
export type Cadence = 'Immediate' | { Hourly: { minute: number } } | { Daily: { minute: number } };
export interface SchedulePolicy {
	destination_id: string;
	destination_revision: number;
	revision: number;
	cadence: Cadence;
	zone: 'SystemLocal' | 'Utc';
	quiet: { start: number; end: number } | null;
	urgent_bypass: boolean;
}
export interface ScheduleState {
	policy: SchedulePolicy;
	deferred: number;
	summarized: number;
	expired: number;
	overflow: number;
}
export interface SchedulePreview {
	next_epoch_ms: number;
	timezone: string;
}
export interface ScheduleTransport {
	read(id: string): Promise<ScheduleState>;
	save(policy: SchedulePolicy, consent: boolean): Promise<SchedulePolicy>;
	preview(policy: SchedulePolicy): Promise<SchedulePreview>;
}
export const nativeScheduleTransport: ScheduleTransport = {
	read: (id) => invoke('cmd_alert_schedule', { id }),
	save: (policy, consent) => invoke('cmd_save_alert_schedule', { policy, consent }),
	preview: (policy) => invoke('cmd_preview_alert_schedule', { policy })
};
export const scheduleLabels = {
	en: {
		title: 'Digests & quiet hours',
		cadence: 'Delivery cadence',
		immediate: 'Immediate',
		hourly: 'Hourly',
		daily: 'Daily',
		time: 'Daily delivery time',
		minute: 'Minute of each hour',
		zone: 'Time zone',
		local: 'System local time',
		utc: 'UTC',
		quiet: 'Enable quiet hours',
		start: 'Quiet hours start',
		end: 'Quiet hours end',
		urgent: 'Allow explicit urgent-rule alerts to bypass digests and quiet hours',
		notice:
			'Missed windows merge into one summary. Unsent items expire after 24 hours. Summaries have at most 50 detailed entries and may be truncated. The app must stay active and unlocked. Changing this policy cancels queued automatic work.',
		dst: 'Missing daylight-saving times use the first valid minute. Repeated times run once, at the earlier occurrence. System timezone changes affect unsealed work.',
		consent: 'I confirm urgent-rule bypass for this destination. Delivery rate limits still apply.',
		next: 'Next eligible delivery',
		preview: 'Preview schedule (no delivery)',
		deferred: 'Deferred',
		summarized: 'Summarized',
		expired: 'Expired',
		overflow: 'Omitted at queue limit',
		save: 'Save schedule',
		cancel: 'Cancel',
		error: 'Schedule operation failed. Check the times or reopen to load current revisions.',
		bypass:
			'This explicit delivery bypasses digest scheduling and quiet hours; delivery rate limits still apply.',
		urgentRule: 'Urgent alert (bypass requires destination permission)',
		urgentHelp:
			'Urgency never comes from message content. Without destination permission, urgent actions follow its routine schedule.'
	},
	'pt-BR': {
		title: 'Resumos e horário de silêncio',
		cadence: 'Frequência de entrega',
		immediate: 'Imediata',
		hourly: 'A cada hora',
		daily: 'Diária',
		time: 'Horário da entrega diária',
		minute: 'Minuto de cada hora',
		zone: 'Fuso horário',
		local: 'Horário local do sistema',
		utc: 'UTC',
		quiet: 'Ativar horário de silêncio',
		start: 'Início do silêncio',
		end: 'Fim do silêncio',
		urgent: 'Permitir que regras urgentes explícitas ignorem resumos e horário de silêncio',
		notice:
			'Janelas perdidas são reunidas em um resumo. Itens não enviados expiram após 24 horas. Resumos têm até 50 entradas detalhadas e podem ser truncados. O aplicativo deve permanecer ativo e desbloqueado. Alterar esta política cancela envios automáticos pendentes.',
		dst: 'Horários inexistentes no horário de verão usam o primeiro minuto válido. Horários repetidos executam uma vez, na primeira ocorrência. Mudanças de fuso do sistema afetam itens ainda não consolidados.',
		consent:
			'Confirmo a exceção para regras urgentes neste destino. Os limites de entrega continuam valendo.',
		next: 'Próxima entrega elegível',
		preview: 'Prévia da programação (sem envio)',
		deferred: 'Adiadas',
		summarized: 'Resumidas',
		expired: 'Expiradas',
		overflow: 'Omitidas pelo limite da fila',
		save: 'Salvar programação',
		cancel: 'Cancelar',
		error: 'A operação falhou. Confira os horários ou reabra para carregar as revisões atuais.',
		bypass:
			'Este envio explícito ignora resumos e horário de silêncio; os limites de entrega continuam valendo.',
		urgentRule: 'Alerta urgente (exceção exige permissão do destino)',
		urgentHelp:
			'A urgência nunca é inferida do conteúdo. Sem permissão do destino, ações urgentes seguem a programação normal.'
	}
};
export function minutes(value: string): number {
	const [hour, minute] = value.split(':').map(Number);
	return hour * 60 + minute;
}
export function time(value: number): string {
	return `${String(Math.floor(value / 60)).padStart(2, '0')}:${String(value % 60).padStart(2, '0')}`;
}
