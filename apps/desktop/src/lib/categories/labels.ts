export const labels = {
	en: {
		more: 'Load more',
		title: 'Local categories',
		all: 'All categories',
		category: {
			Personal: 'People',
			Financial: 'Finance',
			Shopping: 'Purchases',
			Newsletter: 'Newsletters',
			Notifications: 'Notifications',
			Security: 'Security'
		},
		source: { Automatic: 'Automatic', Message: 'Message correction', Sender: 'Sender override' },
		reason: {
			SecuritySignal: 'Security signal',
			FinancialSignal: 'Financial signal',
			PurchaseSignal: 'Purchase signal',
			NewsletterSignal: 'Newsletter signal',
			NotificationSignal: 'Notification signal',
			Fallback: 'Fallback: no stronger signal'
		},
		help: 'Local estimates. People is the fallback; sender addresses do not prove identity. Extraction badges remain independent.',
		durability:
			'This message has no reliable identity. Its correction survives re-sync while cached, but cannot be restored after cache deletion.',
		correct: 'Correct this message',
		reset: 'Reset message correction',
		sender: 'Sender in this account',
		consent:
			'Apply to existing and future cached messages from this exact address in this account. Message corrections take precedence.',
		saveSender: 'Save sender override',
		resetSender: 'Remove sender override',
		overrides: 'Sender overrides',
		error: 'Categories could not be updated. Reload and try again.',
		loading: 'Loading categories…',
		reload: 'Reload',
		empty: 'No sender overrides.',
		unavailable: 'Account unavailable',
		local: 'Category changes stay on this device and do not move messages or run rules.'
	},
	'pt-BR': {
		more: 'Carregar mais',
		title: 'Categorias locais',
		all: 'Todas as categorias',
		category: {
			Personal: 'Pessoas',
			Financial: 'Finanças',
			Shopping: 'Compras',
			Newsletter: 'Boletins',
			Notifications: 'Notificações',
			Security: 'Segurança'
		},
		source: {
			Automatic: 'Automática',
			Message: 'Correção da mensagem',
			Sender: 'Preferência do remetente'
		},
		reason: {
			SecuritySignal: 'Sinal de segurança',
			FinancialSignal: 'Sinal financeiro',
			PurchaseSignal: 'Sinal de compra',
			NewsletterSignal: 'Sinal de boletim',
			NotificationSignal: 'Sinal de notificação',
			Fallback: 'Padrão: nenhum sinal mais forte'
		},
		help: 'Estimativas locais. Pessoas é a categoria padrão; endereços de remetentes não comprovam identidade. Os indicadores de extração são independentes.',
		durability:
			'Esta mensagem não tem identidade confiável. Sua correção resiste à sincronização enquanto estiver no cache, mas não pode ser restaurada após a exclusão do cache.',
		correct: 'Corrigir esta mensagem',
		reset: 'Redefinir correção da mensagem',
		sender: 'Remetente nesta conta',
		consent:
			'Aplicar às mensagens existentes e futuras no cache deste endereço exato nesta conta. Correções individuais têm prioridade.',
		saveSender: 'Salvar preferência do remetente',
		resetSender: 'Remover preferência do remetente',
		overrides: 'Preferências de remetentes',
		error: 'Não foi possível atualizar as categorias. Recarregue e tente novamente.',
		loading: 'Carregando categorias…',
		reload: 'Recarregar',
		empty: 'Nenhuma preferência de remetente.',
		unavailable: 'Conta indisponível',
		local: 'As alterações ficam neste dispositivo e não movem mensagens nem executam regras.'
	}
};
