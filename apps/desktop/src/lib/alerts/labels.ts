import type { ExportField, History } from './types';
interface Labels {
	title: string;
	notice: string;
	add: string;
	name: string;
	local: string;
	webhook: string;
	url: string;
	bearer: string;
	enabled: string;
	consent: string;
	fields: string;
	fieldNames: Record<ExportField, string>;
	save: string;
	cancel: string;
	edit: string;
	remove: string;
	test: string;
	replay: string;
	history: string;
	error: string;
	queued: string;
	confirmTest: string;
	confirmReplay: string;
	send: string;
	unavailable: string;
	empty: string;
	metadata: string;
	preserve: string;
	status: Record<History['status'], string>;
	preview: string;
}
export const labels: Record<'en' | 'pt-BR', Labels> = {
	en: {
		title: 'Alerts',
		notice:
			'Alerts require an active, unlocked app. New-mail rules exclude initial history. Retries may deliver duplicates; acceptance does not prove the recipient saw an alert. Local notices are generic and may appear on the lock screen.',
		add: 'New destination',
		name: 'Destination name',
		local: 'Local notification',
		webhook: 'Public HTTPS webhook',
		url: 'Webhook URL (HTTPS, port 443)',
		bearer: 'Bearer credential (optional)',
		enabled: 'Enabled',
		consent:
			'I confirm this destination and the selected fields. Webhooks export those fields outside this device. Body/excerpt and extracted facts may contain sensitive payment or verification data.',
		fields: 'Exported fields',
		fieldNames: {
			Account: 'Opaque account ID',
			Sender: 'Sender address',
			Subject: 'Subject',
			Excerpt: 'Plain-text excerpt',
			ExtractedFacts: 'Boleto amounts and due dates',
			Attachments: 'Attachment names and MIME types',
			Body: 'Plain-text body (bounded)'
		},
		save: 'Save destination',
		cancel: 'Cancel',
		edit: 'Edit',
		remove: 'Delete',
		test: 'Test delivery',
		replay: 'Replay selected messages',
		history: 'Execution history',
		error:
			'Alert operation failed. Check destination configuration or local notification availability.',
		queued: 'Delivery queued. Keep the app open; refresh history to see the outcome.',
		confirmTest:
			'Send a synthetic test to this enabled destination? A webhook test sends externally.',
		confirmReplay:
			'Send the selected messages to this destination using its selected export fields? This exports data and does not apply mail actions.',
		send: 'Confirm delivery',
		unavailable:
			'Local notifications unavailable or denied. This build uses the Linux desktop notification service.',
		empty: 'No destinations configured.',
		metadata:
			'Always includes schema version, opaque event ID, timestamp and truncation status. Full bodies are off by default. Nothing is fetched to enrich alerts.',
		preserve:
			'Leave URL empty to preserve the saved URL and credential. Entering a new URL replaces both.',
		status: {
			Pending: 'Pending',
			Accepted: 'Accepted',
			Failed: 'Failed',
			Canceled: 'Canceled',
			Expired: 'Expired',
			Unavailable: 'Unavailable'
		},
		preview: 'Payload field preview (no delivery)'
	},
	'pt-BR': {
		title: 'Alertas',
		notice:
			'Alertas exigem o aplicativo ativo e desbloqueado. Regras de novas mensagens excluem o histórico inicial. Tentativas podem gerar duplicatas; aceitação não comprova visualização. Notificações locais são genéricas e podem aparecer na tela bloqueada.',
		add: 'Novo destino',
		name: 'Nome do destino',
		local: 'Notificação local',
		webhook: 'Webhook HTTPS público',
		url: 'URL do webhook (HTTPS, porta 443)',
		bearer: 'Credencial bearer (opcional)',
		enabled: 'Ativo',
		consent:
			'Confirmo este destino e os campos selecionados. Webhooks exportam esses campos para fora deste dispositivo. Corpo/trecho e dados extraídos podem conter informações sensíveis de pagamento ou verificação.',
		fields: 'Campos exportados',
		fieldNames: {
			Account: 'ID opaco da conta',
			Sender: 'Endereço do remetente',
			Subject: 'Assunto',
			Excerpt: 'Trecho em texto simples',
			ExtractedFacts: 'Valores e vencimentos de boletos',
			Attachments: 'Nomes e tipos MIME dos anexos',
			Body: 'Corpo em texto simples (limitado)'
		},
		save: 'Salvar destino',
		cancel: 'Cancelar',
		edit: 'Editar',
		remove: 'Excluir',
		test: 'Testar entrega',
		replay: 'Reenviar mensagens selecionadas',
		history: 'Histórico de execução',
		error:
			'Falha na operação de alerta. Verifique o destino ou a disponibilidade das notificações locais.',
		queued:
			'Entrega na fila. Mantenha o aplicativo aberto; atualize o histórico para ver o resultado.',
		confirmTest:
			'Enviar um teste sintético para este destino ativo? O teste de webhook envia dados externamente.',
		confirmReplay:
			'Enviar as mensagens selecionadas para este destino com os campos de exportação selecionados? Isso exporta dados sem aplicar ações nas mensagens.',
		send: 'Confirmar entrega',
		unavailable:
			'Notificações locais indisponíveis ou negadas. Esta versão usa o serviço de notificações do desktop Linux.',
		empty: 'Nenhum destino configurado.',
		metadata:
			'Sempre inclui versão do esquema, ID opaco do evento, data/hora e indicação de truncamento. Corpos completos ficam desativados por padrão. Nenhum dado é buscado para enriquecer alertas.',
		preserve:
			'Deixe a URL vazia para manter a URL e credencial salvas. Uma nova URL substitui ambas.',
		status: {
			Pending: 'Pendente',
			Accepted: 'Aceito',
			Failed: 'Falhou',
			Canceled: 'Cancelado',
			Expired: 'Expirado',
			Unavailable: 'Indisponível'
		},
		preview: 'Prévia dos campos do payload (sem entrega)'
	}
};
