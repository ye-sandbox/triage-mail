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

	// UI v2 Hub additions
	disclaimerTitle: string;
	dueDateTotalsAria: string;
	overdueHint: string;
	dueTodayHint: string;
	upcomingHint: string;
	clearSearch: string;
	clearFilters: string;
	loadingBills: string;
	extractedOn: (dateIso: string) => string;
	localParser: string;
	copiedToast: string;
	dismissedToast: string;
	configureSchedule: string;
	paused: string;
	activeRulesCount: (count: number) => string;
	presetDaysLabel: (days: number) => string;
	quickPresets: string;
	ledgerAria: string;
	ruleSavedToast: string;
	ruleDeletedToast: string;
	invalidRuleName: string;
	invalidDaysRange: string;
	invalidHourRange: string;
	invalidIntervalRange: string;
}

export const labels: Record<'en' | 'pt-BR', RemindersLabels> = {
	en: {
		title: 'Due Dates & Boletos',
		subtitle: 'Local FEBRABAN boleto extraction ledger and on-device due-date reminders',
		disclaimerTitle: 'Local read-only extraction:',
		disclaimer:
			'Extraction never implies a bill was paid. Extracted deterministically from receipts and boletos locally on-device. Does not constitute banking payment confirmation or initiate transfers.',
		overdue: 'Overdue',
		dueToday: 'Due Today',
		upcoming: 'Upcoming',
		dismissed: 'Dismissed',
		all: 'All',
		filterStatus: 'Filter by status',
		filterAccount: 'Filter by account',
		searchPlaceholder: 'Search bills, senders, barcodes…',
		emptyState: 'No bills or due dates match this filter',
		emptyStateHelp:
			'Invoices, utility bills, and boletos will appear here when detected in your inbox, or try clearing active filters.',
		copyLine: 'Copy barcode',
		copied: 'Copied!',
		viewEmail: 'View Email',
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
		noDestinations: 'No alert destinations configured. Add one in Automation or Settings → Alerts.',
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
		minAmountSummary: (amt) => `min. ${amt}`,

		dueDateTotalsAria: 'Due date totals',
		overdueHint: 'Past due date',
		dueTodayHint: 'Maturing today',
		upcomingHint: 'Scheduled ahead',
		clearSearch: 'Clear search',
		clearFilters: 'Clear Filters',
		loadingBills: 'Loading bills and due dates…',
		extractedOn: (dateIso) => `Extracted ${dateIso}`,
		localParser: 'Local parser',
		copiedToast: 'Digitable line copied',
		dismissedToast: 'Due-date reminder dismissed',
		configureSchedule: 'Configure Reminder Schedule',
		paused: 'Paused',
		activeRulesCount: (count) => `${count} active`,
		presetDaysLabel: (days) => (days === 0 ? 'Due day (0d)' : `${days}d before`),
		quickPresets: 'Quick lead-time presets',
		ledgerAria: 'Extracted boleto ledger',
		ruleSavedToast: 'Reminder rule saved locally',
		ruleDeletedToast: 'Reminder rule deleted',
		invalidRuleName: 'Enter a rule name (1–120 characters).',
		invalidDaysRange: 'Days before deadline must be between 0 and 365.',
		invalidHourRange: 'Notification hour must be between 0 and 23.',
		invalidIntervalRange: 'Unread interval must be between 1 and 8760 hours.'
	},
	'pt-BR': {
		title: 'Contas e Boletos a Vencer',
		subtitle: 'Registro local de boletos FEBRABAN e lembretes programados no dispositivo',
		disclaimerTitle: 'Extração local somente leitura:',
		disclaimer:
			'A extração nunca implica que um boleto foi pago. Valores extraídos deterministicamente de comprovantes e boletos no dispositivo. Não constitui confirmação bancária de pagamento nem inicia transferências.',
		overdue: 'Atrasados',
		dueToday: 'Vencendo Hoje',
		upcoming: 'A Vencer',
		dismissed: 'Dispensados',
		all: 'Todos',
		filterStatus: 'Filtrar por status',
		filterAccount: 'Filtrar por conta',
		searchPlaceholder: 'Buscar contas, remetentes, códigos…',
		emptyState: 'Nenhuma conta ou boleto encontrado neste filtro',
		emptyStateHelp:
			'Faturas, boletos e contas de consumo aparecerão aqui quando detectados na sua caixa de entrada, ou limpe os filtros ativos.',
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
		noDestinations: 'Nenhum destino de alerta configurado. Adicione um em Automação ou Configurações → Alertas.',
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
		minAmountSummary: (amt) => `mín. ${amt}`,

		dueDateTotalsAria: 'Totais por vencimento',
		overdueHint: 'Vencimento ultrapassado',
		dueTodayHint: 'Vence hoje',
		upcomingHint: 'Programados',
		clearSearch: 'Limpar busca',
		clearFilters: 'Limpar Filtros',
		loadingBills: 'Carregando contas e vencimentos…',
		extractedOn: (dateIso) => `Extraído em ${dateIso}`,
		localParser: 'Extrator local',
		copiedToast: 'Linha digitável copiada',
		dismissedToast: 'Lembrete de vencimento dispensado',
		configureSchedule: 'Configurar Agenda de Lembretes',
		paused: 'Pausado',
		activeRulesCount: (count) => `${count} ativa${count === 1 ? '' : 's'}`,
		presetDaysLabel: (days) => (days === 0 ? 'No dia (0d)' : `${days}d antes`),
		quickPresets: 'Atalhos de antecedência',
		ledgerAria: 'Registro de boletos extraídos',
		ruleSavedToast: 'Regra de lembrete salva localmente',
		ruleDeletedToast: 'Regra de lembrete excluída',
		invalidRuleName: 'Informe o nome da regra (1–120 caracteres).',
		invalidDaysRange: 'Dias antes do prazo devem ficar entre 0 e 365.',
		invalidHourRange: 'Horário da notificação deve ficar entre 0 e 23.',
		invalidIntervalRange: 'Intervalo sem leitura deve ficar entre 1 e 8760 horas.'
	}
};
