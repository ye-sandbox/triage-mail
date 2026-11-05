export const labels = {
	en: {
		title: 'Backup & portable definitions',
		backup: 'Create encrypted backup',
		restore: 'Restore encrypted backup',
		help: 'Backups include cached mail, headers, attachment metadata, mailbox state, rules, saved views, local categories, sender choices, dismissals and account photos. Unfetched attachment binaries, external attachment files, models, credentials, network settings, pending actions and delivery history are absent. Mail itself can contain private data.',
		recovery:
			'Restore is available before first setup into a fresh installation only. Existing data and vaults are never replaced. Recovered accounts stay offline; reconnect by adding the same email address in Accounts. Rules, reminders and destinations stay disabled. AI permissions reset. No pending actions replay.',
		pass: 'Backup passphrase (at least 12 characters)',
		confirm: 'Confirm passphrase',
		consent:
			'I understand the included data and omissions. For restore, I will unlock the new isolated local vault with this backup passphrase. A lost passphrase cannot be recovered.',
		newFile:
			'Choose a new filename; existing files are never overwritten. Native file selection requires a Linux desktop portal.',
		busy: 'Working…',
		done: 'Completed.',
		error:
			'Operation failed. Check the passphrase, format, file access and available disk space. Choose a new output filename. Recovery requires a fresh installation.',
		mismatch: 'Passphrases must match and contain at least 12 characters.',
		export: 'Export rules & saved views',
		import: 'Choose definitions to import',
		apply: 'Import reviewed definitions',
		portable:
			'Portable JSON is readable and may contain private names, queries and predicates. It excludes credentials and replaces account/destination IDs with placeholders. Review every definition and map every reference. Rules import disabled with fresh IDs; existing definitions stay intact. All-account definitions continue to cover all accounts.',
		mapping: 'Map references',
		choose: 'Select a target',
		review: 'Review definitions',
		approve:
			'I reviewed these definitions, mappings and all-account scopes. Import without running rules.',
		continue: 'Continue setup',
		welcome: 'Set up local mail or restore a backup',
		restored:
			'Backup restored. Continue and unlock the new local vault using the backup passphrase.',
		disconnected: 'Cached mail only · reconnect by adding this email address'
	},
	'pt-BR': {
		title: 'Backup e definições portáteis',
		backup: 'Criar backup criptografado',
		restore: 'Restaurar backup criptografado',
		help: 'O backup inclui mensagens em cache, cabeçalhos, metadados de anexos, estado das pastas, regras, buscas salvas, categorias locais, escolhas de remetentes, dispensas e fotos de contas. Não inclui binários de anexos não baixados, arquivos externos de anexos, modelos, credenciais, configurações de rede, ações pendentes ou histórico de envios. As mensagens podem conter dados privados.',
		recovery:
			'A restauração só está disponível antes da configuração inicial, em uma instalação nova. Dados e cofres existentes nunca são substituídos. As contas restauradas ficam offline; reconecte adicionando o mesmo endereço em Contas. Regras, lembretes e destinos ficam desativados. Permissões de IA são redefinidas. Ações pendentes não são repetidas.',
		pass: 'Senha do backup (pelo menos 12 caracteres)',
		confirm: 'Confirme a senha',
		consent:
			'Entendo os dados incluídos e as omissões. Na restauração, desbloquearei o novo cofre local isolado com esta senha do backup. Uma senha perdida não pode ser recuperada.',
		newFile:
			'Escolha um nome novo; arquivos existentes nunca são substituídos. A seleção nativa exige um portal de desktop Linux.',
		busy: 'Processando…',
		done: 'Concluído.',
		error:
			'Falha na operação. Verifique a senha, o formato, o acesso ao arquivo e o espaço em disco. Escolha um novo nome de saída. A restauração exige uma instalação nova.',
		mismatch: 'As senhas devem ser iguais e ter pelo menos 12 caracteres.',
		export: 'Exportar regras e buscas salvas',
		import: 'Escolher definições para importar',
		apply: 'Importar definições revisadas',
		portable:
			'O JSON portátil é legível e pode conter nomes, consultas e condições privadas. Exclui credenciais e substitui IDs de contas e destinos por referências. Revise cada definição e mapeie todas as referências. As regras são importadas desativadas com novos IDs; as existentes são preservadas. Definições para todas as contas continuam abrangendo todas as contas.',
		mapping: 'Mapear referências',
		choose: 'Selecione um destino',
		review: 'Revisar definições',
		approve:
			'Revisei as definições, os mapeamentos e os escopos de todas as contas. Importar sem executar regras.',
		continue: 'Continuar configuração',
		welcome: 'Configure o correio local ou restaure um backup',
		restored:
			'Backup restaurado. Continue e desbloqueie o novo cofre local usando a senha do backup.',
		disconnected: 'Apenas cache · reconecte adicionando este endereço'
	}
};
