import type { Field, Outcome, ConditionResult } from './types';
interface Labels {
	message: string;
	title: string;
	notice: string;
	add: string;
	name: string;
	enabled: string;
	all: string;
	any: string;
	except: string;
	field: string;
	value: string;
	remove: string;
	save: string;
	cancel: string;
	edit: string;
	up: string;
	down: string;
	star: string;
	archive: string;
	tags: string;
	stop: string;
	simulate: string;
	selection: string;
	empty: string;
	error: string;
	saved: string;
	createFromMessage: string;
	actions: string;
	cents: string;
	noSelection: string;
	fields: Record<Field, string>;
	outcomes: Record<Outcome | ConditionResult, string>;
}
export const labels: Record<'en' | 'pt-BR', Labels> = {
	en: {
		message: 'Message',
		title: 'Rules',
		notice:
			'Rules run locally on new mail after initial history sync. Saving never changes existing mail. Simulation has no side effects. Sender matches are not proof of identity.',
		add: 'New rule',
		name: 'Rule name',
		enabled: 'Enabled',
		all: 'All conditions',
		any: 'Any condition (optional)',
		except: 'Except when',
		field: 'Field',
		value: 'Value',
		remove: 'Remove',
		save: 'Save rule',
		cancel: 'Cancel draft',
		edit: 'Edit',
		up: 'Move up',
		down: 'Move down',
		star: 'Star',
		archive: 'Archive',
		tags: 'Tags (one per line)',
		stop: 'Stop after this rule matches',
		simulate: 'Simulate selected messages',
		selection: 'Selected messages',
		empty: 'No rules yet.',
		error:
			'Rule operation failed. Check the fields or reload before retrying; the rule may have changed.',
		saved: 'Rule saved.',
		createFromMessage: 'Create rule from message',
		actions: 'Planned actions',
		cents:
			'Amounts use integer BRL cents. Dates use YYYY-MM-DD. Text matches ignore case; domains match exactly unless suffix is selected.',
		noSelection: 'Select a message in the inbox to simulate.',
		fields: {
			Account: 'Account',
			Sender: 'Sender address',
			Domain: 'Sender domain',
			DomainSuffix: 'Domain and subdomains',
			Subject: 'Subject contains',
			Body: 'Body contains',
			AttachmentName: 'Attachment filename contains',
			AttachmentMime: 'Attachment MIME contains',
			AmountAtLeast: 'Amount ≥ (BRL cents)',
			AmountAtMost: 'Amount ≤ (BRL cents)',
			DueOnOrAfter: 'Due on/after',
			DueOnOrBefore: 'Due on/before'
		},
		outcomes: {
			Matched: 'Matched',
			NotMatched: 'Did not match',
			MissingFact: 'Fact unavailable',
			Excluded: 'Excluded by exception',
			Disabled: 'Disabled',
			Stopped: 'Skipped after stop'
		}
	},
	'pt-BR': {
		message: 'Mensagem',
		title: 'Regras',
		notice:
			'As regras são locais e processam novas mensagens após a sincronização inicial. Salvar não altera mensagens existentes. A simulação não realiza ações. Remetente correspondente não comprova identidade.',
		add: 'Nova regra',
		name: 'Nome da regra',
		enabled: 'Ativada',
		all: 'Todas as condições',
		any: 'Qualquer condição (opcional)',
		except: 'Exceto quando',
		field: 'Campo',
		value: 'Valor',
		remove: 'Remover',
		save: 'Salvar regra',
		cancel: 'Cancelar rascunho',
		edit: 'Editar',
		up: 'Mover para cima',
		down: 'Mover para baixo',
		star: 'Marcar estrela',
		archive: 'Arquivar',
		tags: 'Etiquetas (uma por linha)',
		stop: 'Parar quando esta regra corresponder',
		simulate: 'Simular mensagens selecionadas',
		selection: 'Mensagens selecionadas',
		empty: 'Nenhuma regra ainda.',
		error:
			'A operação falhou. Confira os campos ou recarregue antes de tentar novamente; a regra pode ter sido alterada.',
		saved: 'Regra salva.',
		createFromMessage: 'Criar regra da mensagem',
		actions: 'Ações planejadas',
		cents:
			'Valores em centavos inteiros de BRL. Datas no formato AAAA-MM-DD. Texto ignora maiúsculas; domínios são exatos, salvo na opção de subdomínios.',
		noSelection: 'Selecione uma mensagem na caixa de entrada para simular.',
		fields: {
			Account: 'Conta',
			Sender: 'Endereço do remetente',
			Domain: 'Domínio do remetente',
			DomainSuffix: 'Domínio e subdomínios',
			Subject: 'Assunto contém',
			Body: 'Corpo contém',
			AttachmentName: 'Nome do anexo contém',
			AttachmentMime: 'MIME do anexo contém',
			AmountAtLeast: 'Valor ≥ (centavos BRL)',
			AmountAtMost: 'Valor ≤ (centavos BRL)',
			DueOnOrAfter: 'Vencimento em/após',
			DueOnOrBefore: 'Vencimento em/antes'
		},
		outcomes: {
			Matched: 'Correspondeu',
			NotMatched: 'Não correspondeu',
			MissingFact: 'Dado indisponível',
			Excluded: 'Excluída por exceção',
			Disabled: 'Desativada',
			Stopped: 'Ignorada após parada'
		}
	}
};
