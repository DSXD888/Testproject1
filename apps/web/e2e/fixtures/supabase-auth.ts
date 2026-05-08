export const VIEWER_SUPABASE_ENV = {
  NEXT_PUBLIC_SUPABASE_URL: 'https://viewer.example.supabase.co',
  NEXT_PUBLIC_SUPABASE_ANON_KEY: 'viewer-anon-key',
} as const;

export const PRIVILEGED_TOKEN_NAME_PATTERN =
  /service.?role|admin.?key|operator.?token|daemon.?token/i;

export function viewerCredentialNames(): string[] {
  return Object.keys(VIEWER_SUPABASE_ENV);
}

