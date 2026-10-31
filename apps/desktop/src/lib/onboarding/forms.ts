// INVARIANT: Clear only credential values; form.reset() can erase Svelte-owned field defaults.
export function clearCredentialInputs(form: HTMLFormElement | undefined) {
	form?.querySelectorAll<HTMLInputElement>('[data-credential]').forEach((input) => {
		input.value = '';
	});
}
