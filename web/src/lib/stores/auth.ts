import { writable, derived, type Readable } from 'svelte/store';
import { api, type PublicUser } from '$lib/api/client';

export type AuthState = {
	loading: boolean;
	user: PublicUser | null;
	error: string | null;
};

function createAuthStore() {
	const { subscribe, set, update } = writable<AuthState>({
		loading: true,
		user: null,
		error: null
	});

	return {
		subscribe,

		// Initialize auth state by checking if user is logged in
		async init() {
			update((state) => ({ ...state, loading: true, error: null }));

			try {
				const response = await api.getMe();
				set({ loading: false, user: response.user, error: null });
			} catch (e) {
				// Not logged in is not an error
				set({ loading: false, user: null, error: null });
			}
		},

		// Redirect to GitHub login
		login() {
			window.location.href = api.getGitHubLoginUrl();
		},

		// Logout the current user
		async logout() {
			try {
				await api.logout();
				set({ loading: false, user: null, error: null });
			} catch (e) {
				const error = e instanceof Error ? e.message : 'Logout failed';
				update((state) => ({ ...state, error }));
			}
		},

		// Update user info (after profile edit)
		setUser(user: PublicUser | null) {
			update((state) => ({ ...state, user }));
		}
	};
}

export const auth = createAuthStore();

// Derived stores for convenience
export const user: Readable<PublicUser | null> = derived(auth, ($auth) => $auth.user);
export const isLoggedIn: Readable<boolean> = derived(auth, ($auth) => $auth.user !== null);
export const isLoading: Readable<boolean> = derived(auth, ($auth) => $auth.loading);
