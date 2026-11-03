import type { DueDateStatus } from './types';

export interface RemindersLabels {
	title: string;
	subtitle: string;
	disclaimer: string;
	overdue: string;
	dueToday: string;
	upcoming: string;
	dismissed: string;
	all: string;
	filterStatus: string;
	filterAccount: string;
	searchPlaceholder: string;
	emptyState: string;
	emptyStateHelp: string;
	copyLine: string;
	copied: string;
	viewEmail: string;
	dismiss: string;
	dismissAria: string;
	configureReminders: string;
	close: string;
	dueOn: string;
	amount: string;
	noAmount: string;
	daysOverdue: (days: number) => string;
	daysRemaining: (days: number) => string;
	today: string;
	totalAmount: string;
	status: Record<DueDateStatus, string>;

	// Reminders Config Module
	rulesTitle: string;
	rulesNotice: string;
	addRule: string;
	editRule: string;
	ruleName: string;
	ruleNamePlaceholder: string;
	triggerType: string;
	triggerDueDate: string;
	triggerUnread: string;
	daysBefore: string;
	targetHour: string;
	unreadHours: string;
	minAmount: string;
	minAmountPlaceholder: string;
	accountScope: string;
	allAccounts: string;
	destinations: string;
	noDestinations: string;
	save: string;
	delete: string;
	cancel: string;
	edit: string;
	enabled: string;
	disabled: string;
	error: string;
	emptyRules: string;
	ruleSummaryDueDate: (days: number, hour: number) => string;
	ruleSummaryUnread: (hours: number) => string;
	minAmountSummary: (amount: string) => string;
}

export const labels: Record<'en' | 'pt-BR', RemindersLabels> = {
	en: {
		title: 'Due Dates & Boletos',
		subtitle: 'Extracted automatically from invoices and payment slips',
		disclaimer:
			'Extraction never implies a bill was paid. Extracted deterministically from receipts and boletos. Does not constitute banking payment confirmation.',
		overdue: 'Overdue',
		dueToday: 'Due Today',
		upcoming: 'Upcoming',
		dismissed: 'Dismissed',
		all: 'All',
		filterStatus: 'Filter by status',
		filterAccount: 'Filter by account',
		searchPlaceholder: 'Search sender, subject or barcode…',
		emptyState: 'No bills or due dates found',
		emptyStateHelp:
			'Invoices, utility bills, and boletos will appear here when detected in your inbox.',
		copyLine: 'Copy barcode',
		copied: 'Copied!',
		viewEmail: 'View email',
		dismiss: 'Dismiss',
		dismissAria: 'Dismiss reminder',
		configureReminders: 'Configure Reminders',
		close: 'Close',
		dueOn: 'Due',
		amount: 'Amount',
		noAmount: 'Amount not specified',
		daysOverdue: (d) => `${d} day${d === 1 ? '' : 's'} overdue`,
		daysRemaining: (d) => `Due in ${d} day${d === 1 ? '' : 's'}`,
		today: 'Due today',
		totalAmount: 'Total',
		status: {
			Overdue: 'Overdue',
			DueToday: 'Due Today',
			Upcoming: 'Upcoming',
			Dismissed: 'Dismissed'
		},

		rulesTitle: 'Scheduled Reminders',
		rulesNotice:
			'Reminders run locally and dispatch through your configured Alert destinations (local notification, webhook, Telegram, or Discord).',
		addRule: 'New Reminder Rule',
		editRule: 'Edit Reminder Rule',
		ruleName: 'Rule name',
		ruleNamePlaceholder: 'e.g. Boleto 2 days before',
		triggerType: 'Trigger type',
		triggerDueDate: 'Days before due date',
		triggerUnread: 'Hours unread',
		daysBefore: 'Days before deadline',
		targetHour: 'Notification hour (0-23 UTC/local)',
		unreadHours: 'Hours remaining unread',
		minAmount: 'Minimum amount (optional, in R$)',
		minAmountPlaceholder: '0.00',
		accountScope: 'Account filter',
		allAccounts: 'All accounts',
		destinations: 'Alert destinations',
		noDestinations: 'No alert destinations configured. Add one in Settings → Alerts.',
		save: 'Save Rule',
		delete: 'Delete',
		cancel: 'Cancel',
		edit: 'Edit',
		enabled: 'Enabled',
		disabled: 'Disabled',
		error: 'Operation failed. Please try again.',
		emptyRules: 'No scheduled reminder rules configured.',
		ruleSummaryDueDate: (days, hour) =>
			`${days} day${days === 1 ? '' : 's'} before due date at ${hour.toString().padStart(2, '0')}:00`,
		ruleSummaryUnread: (hours) => `Unread for ${hours} hour${hours === 1 ? '' : 's'}`,
		minAmountSummary: (amt) => `min. ${amt}`
	},
	'pt-BR': {
		title: 'Contas e Boletos a Vencer',
		subtitle: 'Extraídos automaticamente de faturas e boletos bancários',
		disclaimer:
			'A extração nunca implica que um boleto foi pago. Valores extraídos deterministicamente de comprovantes e boletos. Não constitui confirmação bancária de pagamento.',
		overdue: 'Atrasados',
		dueToday: 'Vencendo Hoje',
		upcoming: 'A Vencer',
		dismissed: 'Dispensados',
		all: 'Todos',
		filterStatus: 'Filtrar por status',
		filterAccount: 'Filtrar por conta',
		searchPlaceholder: 'Buscar remetente, assunto ou código…',
		emptyState: 'Nenhuma conta ou boleto encontrado',
		emptyStateHelp:
			'Faturas, boletos e contas de consumo aparecerão aqui quando detectados na sua caixa de entrada.',
		copyLine: 'Copiar código de barras',
		copied: 'Copiado!',
		viewEmail: 'Ver e-mail',
		dismiss: 'Dispensar',
		dismissAria: 'Dispensar lembrete',
		configureReminders: 'Configurar Lembretes',
		close: 'Fechar',
		dueOn: 'Vencimento',
		amount: 'Valor',
		noAmount: 'Valor não informado',
		daysOverdue: (d) => `${d} dia${d === 1 ? '' : 's'} em atraso`,
		daysRemaining: (d) => `Vence em ${d} dia${d === 1 ? '' : 's'}`,
		today: 'Vence hoje',
		totalAmount: 'Total',
		status: {
			Overdue: 'Atrasado',
			DueToday: 'Vencendo Hoje',
			Upcoming: 'A Vencer',
			Dismissed: 'Dispensado'
		},

		rulesTitle: 'Lembretes Programados',
		rulesNotice:
			'Os lembretes são executados localmente e disparados pelos destinos configurados de Alerta (notificação local, webhook, Telegram ou Discord).',
		addRule: 'Nova Regra de Lembrete',
		editRule: 'Editar Regra de Lembrete',
		ruleName: 'Nome da regra',
		ruleNamePlaceholder: 'ex: Boleto 2 dias antes',
		triggerType: 'Tipo de gatilho',
		triggerDueDate: 'Dias antes do vencimento',
		triggerUnread: 'Horas sem leitura',
		daysBefore: 'Dias antes do prazo',
		targetHour: 'Horário da notificação (hora 0-23)',
		unreadHours: 'Horas aguardando leitura',
		minAmount: 'Valor mínimo (opcional, em R$)',
		minAmountPlaceholder: '0,00',
		accountScope: 'Filtro por conta',
		allAccounts: 'Todas as contas',
		destinations: 'Destinos de alerta',
		noDestinations: 'Nenhum destino de alerta configurado. Adicione um em Configurações → Alertas.',
		save: 'Salvar Regra',
		delete: 'Excluir',
		cancel: 'Cancelar',
		edit: 'Editar',
		enabled: 'Ativo',
		disabled: 'Desativado',
		error: 'Falha na operação. Tente novamente.',
		emptyRules: 'Nenhuma regra de lembrete programada.',
		ruleSummaryDueDate: (days, hour) =>
			`${days} dia${days === 1 ? '' : 's'} antes do vencimento às ${hour.toString().padStart(2, '0')}:00`,
		ruleSummaryUnread: (hours) => `Sem leitura por ${hours} hora${hours === 1 ? '' : 's'}`,
		minAmountSummary: (amt) => `mín. ${amt}`
	}
};
