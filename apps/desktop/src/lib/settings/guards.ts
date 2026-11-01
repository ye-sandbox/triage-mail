/**
 * Unsaved-form and pending-operation guard for Settings modal modules.
 * INVARIANT: Never copies credential inputs into drafts or storage.
 * Tracks boolean dirty/busy flags without external dependencies.
 */

export class SettingsFormGuard {
	dirty = false;
	busy = false;

	markDirty(dirty = true): void {
		this.dirty = dirty;
	}

	setBusy(busy: boolean): void {
		this.busy = busy;
	}

	canProceed(): boolean {
		return !this.busy && !this.dirty;
	}

	isBlocked(): boolean {
		return this.busy;
	}

	needsConfirmation(): boolean {
		return !this.busy && this.dirty;
	}

	reset(): void {
		this.dirty = false;
		this.busy = false;
	}
}
