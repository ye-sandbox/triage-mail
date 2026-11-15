import type { SupportedLocale } from '../i18n/types';

export interface AutomationLabels {
	pageTitle: string;
	pageSubtitle: string;
	rulesActiveMetric: (active: number, total: number) => string;
	destinationsMetric: (count: number) => string;
	queuedSendersMetric: (count: number) => string;
	tabs: {
		rules: string;
		alerts: string;
		governance: string;
	};
	rulesPane: {
		pipelineTitle: string;
		pipelineSubtitle: string;
		newRule: string;
		emptyList: string;
		condsLabel: (count: number) => string;
		actionsLabel: (count: number) => string;
		matchesLabel: (count: number) => string;
		stopProcessing: string;
		editRule: string;
		moveUp: string;
		moveDown: string;
		toggleRule: (name: string) => string;
		deleteRule: string;
		createRule: string;
		saveRule: string;
		cancelDraft: string;
		ruleName: string;
		ruleNamePlaceholder: string;
		enabledLabel: string;
		groupAll: string;
		groupAllLogic: string;
		groupAny: string;
		groupAnyLogic: string;
		groupExcept: string;
		groupExceptLogic: string;
		addCondition: string;
		noGroupConditions: (group: string) => string;
		conditionFieldAria: string;
		conditionValueAria: string;
		removeConditionAria: string;
		actionStack: (count: number) => string;
		addActionHint: string;
		actionTypeAria: string;
		targetDestinationAria: string;
		removeActionAria: string;
		noParamRequired: string;
		noDestinationsAvailable: string;
		unavailableDestination: string;
		ruleSavedToast: (name: string) => string;
		ruleCreatedToast: (name: string) => string;
		ruleDeletedToast: string;
		ruleToggledToast: (name: string, enabled: boolean) => string;
		ruleMovedToast: (name: string, pos: number) => string;
		centsHelper: string;
		validationMissingCondition: string;
		validationMissingAction: string;
		validationTooManyConditions: string;
		validationInvalidTag: string;
		validationInvalidCondition: string;
	};
	simulator: {
		title: string;
		subtitle: string;
		ranAt: (time: string) => string;
		runButton: string;
		running: string;
		emptyPrompt: string;
		selectedCount: (count: number) => string;
		noMessages: string;
		planPrefix: string;
		plannedActions: string;
		shortCircuitedBy: string;
		ruleLabel: string;
		allLabel: string;
		anyLabel: string;
		exceptLabel: string;
		simulatedToast: (total: number, matched: number) => string;
	};
	alertsPane: {
		title: string;
		subtitle: string;
		newDestination: string;
		toggleDestinationAria: (name: string) => string;
		credentialsSealed: string;
		credentialsNone: string;
		fieldsCount: (count: number) => string;
		localNoExport: string;
		scheduleButton: string;
		scheduleQuiet: (start: string, end: string) => string;
		scheduleOff: string;
		historyTitle: string;
		historySubtitle: string;
		refreshHistory: string;
		emptyHistory: string;
		colEventId: string;
		colDestination: string;
		colStatus: string;
		colAttempts: string;
		colTimestamp: string;
		colDetail: string;
		colAction: string;
		replayButton: string;
		dialogTitleNew: string;
		dialogTitleEdit: (rev: number) => string;
		closeDialogAria: string;
		tabDestination: string;
		tabSchedule: string;
		kindLabel: string;
		kindLocal: string;
		kindWebhook: string;
		kindTelegram: string;
		kindDiscord: string;
		originLabel: string;
		originPlaceholder: string;
		sealedPlaceholder: string;
		originHint: string;
		saveScheduleRequired: string;
		validationNameRequired: string;
		validationFieldsRequired: string;
		validationConsentRequired: string;
		validationWebhookInvalid: string;
		validationStartEndEqual: string;
		destinationCreatedToast: (name: string) => string;
		destinationUpdatedToast: (name: string, rev: number) => string;
		destinationDeletedToast: (name: string) => string;
		destinationToggledToast: (name: string, enabled: boolean) => string;
		scheduleSavedToast: string;
	};
	reviewPane: {
		title: string;
		subtitle: string;
		filterAll: string;
		filterPending: string;
		filterDeclined: string;
		filterAccepted: string;
		filterKnown: string;
		noSendersMatch: string;
		msgsCount: (count: number) => string;
		toggleEnableAria: string;
		toggleHideAria: string;
		acceptButton: string;
		declineButton: string;
		pendingButton: string;
		decisionToast: (name: string, state: string) => string;
		preferencesSavedToast: string;
	};
	categoriesPane: {
		title: string;
		subtitle: string;
		colSender: string;
		colReason: string;
		colAuto: string;
		colOverride: string;
		colAction: string;
		overrideSelectAria: (sender: string) => string;
		resetButton: string;
		updatedToast: (sender: string, category: string) => string;
		removedToast: (sender: string) => string;
		provenanceSender: string;
	};
}

export const automationLabels: Record<SupportedLocale, AutomationLabels> = {
	en: {
		pageTitle: 'Automation & Triage Governance',
		pageSubtitle:
			'Deterministic local rules, dry-run simulation, outbound consent policies, and first-contact sender screening.',
		rulesActiveMetric: (active, total) => `${active}/${total} rules active`,
		destinationsMetric: (count) => `${count} destinations`,
		queuedSendersMetric: (count) => `${count} queued senders`,
		tabs: {
			rules: 'Deterministic Rules',
			alerts: 'Alert Destinations & Delivery',
			governance: 'Senders & Overrides'
		},
		rulesPane: {
			pipelineTitle: 'Ordered Rule Pipeline',
			pipelineSubtitle: 'Evaluated top-to-bottom.',
			newRule: 'New Rule',
			emptyList: 'No rules configured yet.',
			condsLabel: (count) => `${count} conds`,
			actionsLabel: (count) => `${count} actions`,
			matchesLabel: (count) => `${count} matches`,
			stopProcessing: 'stop_processing',
			editRule: 'Edit rule',
			moveUp: 'Move rule up',
			moveDown: 'Move rule down',
			toggleRule: (name) => `Toggle ${name}`,
			deleteRule: 'Delete rule',
			createRule: 'Create Rule',
			saveRule: 'Save Rule',
			cancelDraft: 'Cancel',
			ruleName: 'Rule Name',
			ruleNamePlaceholder: 'e.g. FEBRABAN Boletos & Utility Invoices',
			enabledLabel: 'Enabled',
			groupAll: 'ALL',
			groupAllLogic: 'Every condition must match',
			groupAny: 'ANY',
			groupAnyLogic: 'At least one condition must match (optional)',
			groupExcept: 'EXCEPT',
			groupExceptLogic: 'Exclude message if any condition matches',
			addCondition: 'Add Condition',
			noGroupConditions: (group) => `No ${group} conditions configured.`,
			conditionFieldAria: 'Condition field',
			conditionValueAria: 'Condition value',
			removeConditionAria: 'Remove condition',
			actionStack: (count) => `Action Stack (${count})`,
			addActionHint: 'Add at least one deterministic action.',
			actionTypeAria: 'Action type',
			targetDestinationAria: 'Target destination',
			removeActionAria: 'Remove action',
			noParamRequired: 'No parameter required',
			noDestinationsAvailable: 'No destinations configured yet',
			unavailableDestination: 'Unavailable destination',
			ruleSavedToast: (name) => `Saved changes to "${name}"`,
			ruleCreatedToast: (name) => `Created rule "${name}"`,
			ruleDeletedToast: 'Rule removed',
			ruleToggledToast: (name, enabled) => `${name}: ${enabled ? 'Enabled' : 'Disabled'}`,
			ruleMovedToast: (name, pos) => `Moved "${name}" to position ${String(pos).padStart(2, '0')}`,
			centsHelper:
				'Amounts use integer BRL cents (e.g. 5000 = R$ 50,00). Dates use YYYY-MM-DD. Text ignores case.',
			validationMissingCondition: 'Add at least one condition in ALL or ANY.',
			validationMissingAction: 'Add at least one action.',
			validationTooManyConditions: 'A rule can contain at most 64 total conditions.',
			validationInvalidTag: 'Tag labels must be non-empty and at most 80 characters.',
			validationInvalidCondition: 'Fill in valid values for every condition before saving.'
		},
		simulator: {
			title: 'Dry-Run Rule Simulator',
			subtitle:
				'Evaluates draft conditions and ordered stop_processing precedence without mutating mailbox state.',
			ranAt: (time) => `Ran ${time}`,
			runButton: 'Simulate selected messages',
			running: 'Simulating…',
			emptyPrompt:
				'Click Simulate selected messages to inspect per-condition evaluations and execution plans.',
			selectedCount: (count) => `${count} target message(s)`,
			noMessages: 'Select a message in the inbox to simulate.',
			planPrefix: 'Plan:',
			plannedActions: 'Planned actions:',
			shortCircuitedBy: 'Short-circuited by earlier rule:',
			ruleLabel: 'Rule',
			allLabel: 'ALL',
			anyLabel: 'ANY',
			exceptLabel: 'EXCEPT',
			simulatedToast: (total, matched) =>
				`Simulated against ${total} message(s) (${matched} matched)`
		},
		alertsPane: {
			title: 'Alert Destinations & Field Consent',
			subtitle:
				'Every outbound destination enforces explicit field allow-lists, quiet-hours policies, and sealed credentials.',
			newDestination: 'New Destination',
			toggleDestinationAria: (name) => `Toggle ${name}`,
			credentialsSealed: 'has_credentials: sealed',
			credentialsNone: 'has_credentials: none',
			fieldsCount: (count) => `Fields (${count})`,
			localNoExport: 'Local OS notification (zero external export fields)',
			scheduleButton: 'Schedule & Quiet Hours',
			scheduleQuiet: (start, end) => `${start}–${end}`,
			scheduleOff: 'Off',
			historyTitle: 'Delivery History',
			historySubtitle: 'Auditable local log of outbound alert dispatches and retry attempts.',
			refreshHistory: 'Refresh',
			emptyHistory: 'No delivery events recorded yet.',
			colEventId: 'event_id',
			colDestination: 'destination',
			colStatus: 'status',
			colAttempts: 'attempts',
			colTimestamp: 'timestamp',
			colDetail: 'detail',
			colAction: 'action',
			replayButton: 'Replay',
			dialogTitleNew: 'New Alert Destination',
			dialogTitleEdit: (rev) => `Edit Destination · rev ${rev}`,
			closeDialogAria: 'Close dialog',
			tabDestination: 'Destination & Consent',
			tabSchedule: 'Digests & Quiet Hours',
			kindLabel: 'Transport Kind',
			kindLocal: 'Local (OS Notification)',
			kindWebhook: 'Webhook (HTTPS :443 public DNS)',
			kindTelegram: 'Telegram (api.telegram.org)',
			kindDiscord: 'Discord (discord.com)',
			originLabel: 'Origin',
			originPlaceholder: 'https://hooks.ledger.example.org/ingest',
			sealedPlaceholder: 'Sealed in local vault (enter to rotate)',
			originHint: 'Raw secrets are never displayed or returned after sealing.',
			saveScheduleRequired: 'Save the destination first to configure its schedule policy.',
			validationNameRequired: 'Destination name is required (1–120 characters).',
			validationFieldsRequired: 'Select at least one export field for external destinations.',
			validationConsentRequired: 'Explicit export consent is required to enable this destination.',
			validationWebhookInvalid:
				'Webhook must use a public HTTPS domain on port 443 (localhost and IP literals are blocked).',
			validationStartEndEqual: 'Quiet hours start and end times cannot be equal.',
			destinationCreatedToast: (name) => `Created ${name}`,
			destinationUpdatedToast: (name, rev) => `Updated ${name} (rev ${rev})`,
			destinationDeletedToast: (name) => `Deleted ${name}`,
			destinationToggledToast: (name, enabled) => `${name}: ${enabled ? 'Active' : 'Paused'}`,
			scheduleSavedToast: 'Schedule policy saved'
		},
		reviewPane: {
			title: 'New Senders Screener Queue',
			subtitle: 'First-contact senders wait in screening before entering your primary unified inbox.',
			filterAll: 'All',
			filterPending: 'Pending',
			filterDeclined: 'Declined',
			filterAccepted: 'Accepted',
			filterKnown: 'Known',
			noSendersMatch: 'No senders match this filter.',
			msgsCount: (count) => `${count} msgs`,
			toggleEnableAria: 'Toggle enable sender review',
			toggleHideAria: 'Toggle hide pending and declined in inbox',
			acceptButton: 'Accept',
			declineButton: 'Decline',
			pendingButton: 'Pending',
			decisionToast: (name, state) => `${name}: ${state}`,
			preferencesSavedToast: 'Sender review preferences saved.'
		},
		categoriesPane: {
			title: 'Local Category Overrides',
			subtitle:
				'Per-sender local classification overrides that take precedence over automatic heuristic signals.',
			colSender: 'Sender / Account',
			colReason: 'Signal / Provenance',
			colAuto: 'Auto Category',
			colOverride: 'User Override',
			colAction: 'Action',
			overrideSelectAria: (sender) => `Category override for ${sender}`,
			resetButton: 'Reset',
			updatedToast: (sender, category) => `Updated ${sender} -> ${category}`,
			removedToast: (sender) => `Removed override for ${sender}`,
			provenanceSender: 'SenderOverride'
		}
	},
	'pt-BR': {
		pageTitle: 'Automação e Governança de Triagem',
		pageSubtitle:
			'Regras locais determinísticas, simulação a seco, políticas de consentimento e triagem de novos remetentes.',
		rulesActiveMetric: (active, total) => `${active}/${total} regras ativas`,
		destinationsMetric: (count) => `${count} destinos`,
		queuedSendersMetric: (count) => `${count} remetentes na fila`,
		tabs: {
			rules: 'Regras Determinísticas',
			alerts: 'Destinos de Alerta e Entrega',
			governance: 'Remetentes e Categorias'
		},
		rulesPane: {
			pipelineTitle: 'Pipeline Ordenado de Regras',
			pipelineSubtitle: 'Avaliado de cima para baixo.',
			newRule: 'Nova Regra',
			emptyList: 'Nenhuma regra configurada ainda.',
			condsLabel: (count) => `${count} cond.`,
			actionsLabel: (count) => `${count} ações`,
			matchesLabel: (count) => `${count} corresp.`,
			stopProcessing: 'stop_processing',
			editRule: 'Editar regra',
			moveUp: 'Mover regra para cima',
			moveDown: 'Mover regra para baixo',
			toggleRule: (name) => `Alternar ${name}`,
			deleteRule: 'Excluir regra',
			createRule: 'Criar Regra',
			saveRule: 'Salvar Regra',
			cancelDraft: 'Cancelar',
			ruleName: 'Nome da Regra',
			ruleNamePlaceholder: 'ex. Boletos FEBRABAN e Faturas',
			enabledLabel: 'Ativada',
			groupAll: 'TODAS (ALL)',
			groupAllLogic: 'Todas as condições devem corresponder',
			groupAny: 'QUALQUER (ANY)',
			groupAnyLogic: 'Pelo menos uma condição deve corresponder (opcional)',
			groupExcept: 'EXCETO (EXCEPT)',
			groupExceptLogic: 'Exclui a mensagem se alguma condição corresponder',
			addCondition: 'Adicionar Condição',
			noGroupConditions: (group) => `Nenhuma condição ${group} configurada.`,
			conditionFieldAria: 'Campo da condição',
			conditionValueAria: 'Valor da condição',
			removeConditionAria: 'Remover condição',
			actionStack: (count) => `Pilha de Ações (${count})`,
			addActionHint: 'Adicione pelo menos uma ação determinística.',
			actionTypeAria: 'Tipo de ação',
			targetDestinationAria: 'Destino do alerta',
			removeActionAria: 'Remover ação',
			noParamRequired: 'Nenhum parâmetro necessário',
			noDestinationsAvailable: 'Nenhum destino configurado ainda',
			unavailableDestination: 'Destino indisponível',
			ruleSavedToast: (name) => `Alterações salvas em "${name}"`,
			ruleCreatedToast: (name) => `Regra "${name}" criada`,
			ruleDeletedToast: 'Regra removida',
			ruleToggledToast: (name, enabled) => `${name}: ${enabled ? 'Ativada' : 'Desativada'}`,
			ruleMovedToast: (name, pos) =>
				`"${name}" movida para a posição ${String(pos).padStart(2, '0')}`,
			centsHelper:
				'Valores usam centavos inteiros de BRL (ex. 5000 = R$ 50,00). Datas em AAAA-MM-DD. Texto ignora maiúsculas.',
			validationMissingCondition: 'Adicione pelo menos uma condição em TODAS ou QUALQUER.',
			validationMissingAction: 'Adicione pelo menos uma ação.',
			validationTooManyConditions: 'Uma regra pode ter no máximo 64 condições no total.',
			validationInvalidTag: 'Etiquetas devem ser não vazias e ter até 80 caracteres.',
			validationInvalidCondition: 'Preencha valores válidos para todas as condições antes de salvar.'
		},
		simulator: {
			title: 'Simulador de Regras (Dry-Run)',
			subtitle:
				'Avalia as condições do rascunho e a precedência de stop_processing sem alterar o estado da caixa.',
			ranAt: (time) => `Executado às ${time}`,
			runButton: 'Simular mensagens selecionadas',
			running: 'Simulando…',
			emptyPrompt:
				'Clique em Simular mensagens selecionadas para inspecionar cada condição e o plano de execução.',
			selectedCount: (count) => `${count} mensagem(ns) alvo`,
			noMessages: 'Selecione uma mensagem na caixa de entrada para simular.',
			planPrefix: 'Plano:',
			plannedActions: 'Ações planejadas:',
			shortCircuitedBy: 'Interrompida por regra anterior:',
			ruleLabel: 'Regra',
			allLabel: 'TODAS',
			anyLabel: 'QUALQUER',
			exceptLabel: 'EXCETO',
			simulatedToast: (total, matched) =>
				`Simulado em ${total} mensagem(ns) (${matched} correspondência(s))`
		},
		alertsPane: {
			title: 'Destinos de Alerta e Consentimento',
			subtitle:
				'Cada destino externo exige lista explícita de campos exportados, horário de silêncio e credenciais seladas.',
			newDestination: 'Novo Destino',
			toggleDestinationAria: (name) => `Alternar ${name}`,
			credentialsSealed: 'has_credentials: selado',
			credentialsNone: 'has_credentials: nenhum',
			fieldsCount: (count) => `Campos (${count})`,
			localNoExport: 'Notificação local do sistema (zero campos exportados)',
			scheduleButton: 'Programação e Silêncio',
			scheduleQuiet: (start, end) => `${start}–${end}`,
			scheduleOff: 'Desligado',
			historyTitle: 'Histórico de Entregas',
			historySubtitle: 'Registro local auditável de envios de alerta e tentativas de reenvio.',
			refreshHistory: 'Atualizar',
			emptyHistory: 'Nenhum evento de entrega registrado ainda.',
			colEventId: 'event_id',
			colDestination: 'destino',
			colStatus: 'status',
			colAttempts: 'tentativas',
			colTimestamp: 'data/hora',
			colDetail: 'detalhe',
			colAction: 'ação',
			replayButton: 'Reenviar',
			dialogTitleNew: 'Novo Destino de Alerta',
			dialogTitleEdit: (rev) => `Editar Destino · rev ${rev}`,
			closeDialogAria: 'Fechar janela',
			tabDestination: 'Destino e Consentimento',
			tabSchedule: 'Resumos e Silêncio',
			kindLabel: 'Tipo de Transporte',
			kindLocal: 'Local (Notificação do Sistema)',
			kindWebhook: 'Webhook (HTTPS :443 DNS público)',
			kindTelegram: 'Telegram (api.telegram.org)',
			kindDiscord: 'Discord (discord.com)',
			originLabel: 'Origem',
			originPlaceholder: 'https://hooks.ledger.example.org/ingest',
			sealedPlaceholder: 'Selado no cofre local (digite para rotacionar)',
			originHint: 'Segredos nunca são exibidos ou retornados após selagem.',
			saveScheduleRequired: 'Salve o destino primeiro para configurar a programação.',
			validationNameRequired: 'O nome do destino é obrigatório (1–120 caracteres).',
			validationFieldsRequired: 'Selecione ao menos um campo exportado para destinos externos.',
			validationConsentRequired:
				'O consentimento explícito de exportação é obrigatório para ativar este destino.',
			validationWebhookInvalid:
				'O webhook deve usar um domínio HTTPS público na porta 443 (localhost e IPs são bloqueados).',
			validationStartEndEqual: 'Os horários de início e fim do silêncio não podem ser iguais.',
			destinationCreatedToast: (name) => `Destino ${name} criado`,
			destinationUpdatedToast: (name, rev) => `Destino ${name} atualizado (rev ${rev})`,
			destinationDeletedToast: (name) => `Destino ${name} excluído`,
			destinationToggledToast: (name, enabled) => `${name}: ${enabled ? 'Ativo' : 'Pausado'}`,
			scheduleSavedToast: 'Política de programação salva'
		},
		reviewPane: {
			title: 'Fila de Revisão de Novos Remetentes',
			subtitle:
				'Remetentes de primeiro contato aguardam triagem antes de entrar na caixa unificada principal.',
			filterAll: 'Todos',
			filterPending: 'Pendentes',
			filterDeclined: 'Recusados',
			filterAccepted: 'Aceitos',
			filterKnown: 'Conhecidos',
			noSendersMatch: 'Nenhum remetente corresponde a este filtro.',
			msgsCount: (count) => `${count} msgs`,
			toggleEnableAria: 'Alternar ativação da revisão de remetentes',
			toggleHideAria: 'Alternar ocultação de pendentes e recusados na Caixa de entrada',
			acceptButton: 'Aceitar',
			declineButton: 'Recusar',
			pendingButton: 'Pendente',
			decisionToast: (name, state) => `${name}: ${state}`,
			preferencesSavedToast: 'Preferências de revisão salvas.'
		},
		categoriesPane: {
			title: 'Substituições Locais de Categoria',
			subtitle:
				'Substituições locais por remetente que têm precedência sobre os sinais heurísticos automáticos.',
			colSender: 'Remetente / Conta',
			colReason: 'Sinal / Origem',
			colAuto: 'Cat. Automática',
			colOverride: 'Substituição',
			colAction: 'Ação',
			overrideSelectAria: (sender) => `Categoria para ${sender}`,
			resetButton: 'Redefinir',
			updatedToast: (sender, category) => `${sender} atualizado -> ${category}`,
			removedToast: (sender) => `Preferência removida para ${sender}`,
			provenanceSender: 'Preferência de Remetente'
		}
	}
};
