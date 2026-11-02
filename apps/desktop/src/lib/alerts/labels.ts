import type { ExportField, History } from './types';
interface Labels {
	token: string;
	chat: string;
	telegramHelp: string;
	discordHelp: string;
	revoke: string;
	complete: string;
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
		token: 'Bot token',
		chat: 'Numeric chat ID (confirm the intended recipient)',
		telegramHelp:
			'Create a bot with Telegram BotFather. Start the bot in the intended chat, or add it to your group/channel with permission to post. Enter its token and the exact numeric chat ID from your own Telegram administration tools. This app does not discover chats. Messages use plain text, disable link previews and may be truncated.',
		discordHelp:
			'In Discord channel settings → Integrations → Webhooks, create a webhook and copy its canonical https://discord.com/api/webhooks/ URL. Confirm the intended channel there. Mentions and embeds are suppressed; long messages are truncated.',
		revoke:
			'Disable or delete here to stop local dispatch. To revoke the credential, use BotFather or delete the webhook in Discord. Previously accepted messages cannot be recalled.',
		complete: 'Enter both bot token and numeric chat ID to replace the saved recipient.',
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
			'I confirm this destination and the selected fields. External destinations export those fields outside this device. Body/excerpt and extracted facts may contain sensitive payment or verification data.',
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
			'Send a synthetic test to this enabled destination? An external destination test sends outside this device.',
		confirmReplay:
			'Send the selected messages to this destination using its selected export fields? This exports data and does not apply mail actions.',
		send: 'Confirm delivery',
		unavailable:
			'Local notifications unavailable or denied. This build uses the Linux desktop notification service.',
		empty: 'No destinations configured.',
		metadata:
			'Always includes schema version, opaque event ID, timestamp and truncation status. Full bodies are off by default. Nothing is fetched to enrich alerts.',
		preserve:
			'Leave secret inputs empty to preserve saved material for the same destination type. Enter replacement material in full.',
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
		token: 'Token do bot',
		chat: 'ID numérico do chat (confirme o destinatário)',
		telegramHelp:
			'Crie um bot com o BotFather do Telegram. Inicie o bot no chat desejado ou adicione-o ao grupo/canal com permissão para publicar. Informe o token e o ID numérico exato do chat obtido nas suas ferramentas de administração do Telegram. Este aplicativo não descobre chats. Mensagens usam texto simples, desativam prévias de links e podem ser truncadas.',
		discordHelp:
			'Nas configurações do canal Discord → Integrações → Webhooks, crie um webhook e copie a URL canônica https://discord.com/api/webhooks/. Confirme o canal desejado lá. Menções e incorporações são suprimidas; mensagens longas são truncadas.',
		revoke:
			'Desative ou exclua aqui para parar os envios locais. Para revogar a credencial, use o BotFather ou exclua o webhook no Discord. Mensagens já aceitas não podem ser recolhidas.',
		complete: 'Informe token do bot e ID numérico do chat para substituir o destinatário salvo.',
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
			'Confirmo este destino e os campos selecionados. Destinos externos exportam esses campos para fora deste dispositivo. Corpo/trecho e dados extraídos podem conter informações sensíveis de pagamento ou verificação.',
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
			'Enviar um teste sintético para este destino ativo? O teste de destino externo envia dados para fora deste dispositivo.',
		confirmReplay:
			'Enviar as mensagens selecionadas para este destino com os campos de exportação selecionados? Isso exporta dados sem aplicar ações nas mensagens.',
		send: 'Confirmar entrega',
		unavailable:
			'Notificações locais indisponíveis ou negadas. Esta versão usa o serviço de notificações do desktop Linux.',
		empty: 'Nenhum destino configurado.',
		metadata:
			'Sempre inclui versão do esquema, ID opaco do evento, data/hora e indicação de truncamento. Corpos completos ficam desativados por padrão. Nenhum dado é buscado para enriquecer alertas.',
		preserve:
			'Deixe os campos secretos vazios para manter os dados salvos do mesmo tipo de destino. Informe todos os dados para substituí-los.',
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
