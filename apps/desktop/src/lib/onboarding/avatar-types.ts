export type AccountAvatar = {
	bytes: number[];
	mime_type: string;
	width: number;
	height: number;
};
export interface AvatarTransport {
	get(accountId: string): Promise<AccountAvatar | null>;
	set(accountId: string, bytes: number[]): Promise<AccountAvatar>;
	remove(accountId: string): Promise<void>;
}
