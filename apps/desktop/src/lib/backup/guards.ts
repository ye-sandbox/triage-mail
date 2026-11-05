import { SettingsFormGuard } from '../settings/guards';

// INVARIANT: Completing one form cannot clear another form's unsaved credentials
// or preview. Both forms share the parent's exclusion gate for pending operations.
export function transferGuards(parent: SettingsFormGuard) {
	const children: SettingsFormGuard[] = [];
	function refresh() {
		parent.markDirty(children.some((child) => child.dirty));
		parent.setBusy(children.some((child) => child.busy));
	}
	class ChildGuard extends SettingsFormGuard {
		override markDirty(dirty = true) {
			super.markDirty(dirty);
			refresh();
		}
		override setBusy(busy: boolean) {
			super.setBusy(busy);
			refresh();
		}
		override reset() {
			super.reset();
			refresh();
		}
		override isBlocked() {
			return parent.isBlocked();
		}
	}
	const backup = new ChildGuard();
	const portable = new ChildGuard();
	children.push(backup, portable);
	return { backup, portable };
}
