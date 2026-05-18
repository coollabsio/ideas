export interface User {
  login: string;
  avatarUrl: string;
}

export interface Idea {
  id: string;
  title: string;
  bodyText: string;
  upvoteCount: number;
  viewerHasUpvoted: boolean;
  viewerCanEdit: boolean;
  viewerCanDelete: boolean;
  viewerCanClose: boolean;
  author: User;
  createdAt: string;
  updatedAt: string;
  closed: boolean;
}

export interface MeResponse {
  user: User | null;
  csrfToken?: string | null;
}

export const meKeys = {
  all: ['me'] as const,
  current: () => [...meKeys.all, 'current'] as const
};

export const ideaKeys = {
  all: ['ideas'] as const,
  lists: () => [...ideaKeys.all, 'list'] as const
};

export function meQueryOptions() {
  return {
    queryKey: meKeys.current(),
    queryFn: fetchMe
  };
}

export function ideasQueryOptions() {
  return {
    queryKey: ideaKeys.lists(),
    queryFn: fetchIdeas
  };
}

export async function fetchMe(): Promise<MeResponse> {
  const res = await fetch('/api/me', { credentials: 'same-origin', cache: 'no-store' });
  if (!res.ok) throw new Error(`Could not load session: HTTP ${res.status}`);
  return res.json();
}

export async function fetchIdeas(): Promise<Idea[]> {
  const res = await fetch('/api/ideas', { credentials: 'same-origin', cache: 'no-store' });
  if (!res.ok) throw new Error(`Could not load ideas: HTTP ${res.status}`);
  return res.json();
}

export async function createIdea(title: string, body: string, csrfToken: string): Promise<Idea> {
  const res = await fetch('/api/ideas', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'content-type': 'application/json', 'x-csrf-token': csrfToken },
    body: JSON.stringify({ title, body })
  });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function updateIdea(id: string, title: string, body: string, csrfToken: string): Promise<Idea> {
  const res = await fetch(`/api/ideas/${id}`, {
    method: 'PATCH',
    credentials: 'same-origin',
    headers: { 'content-type': 'application/json', 'x-csrf-token': csrfToken },
    body: JSON.stringify({ title, body })
  });
  if (res.status === 401) window.location.href = '/api/auth/login';
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function setIdeaStatus(id: string, closed: boolean, csrfToken: string): Promise<Idea> {
  const res = await fetch(`/api/ideas/${id}/status`, {
    method: 'PATCH',
    credentials: 'same-origin',
    headers: { 'content-type': 'application/json', 'x-csrf-token': csrfToken },
    body: JSON.stringify({ closed })
  });
  if (res.status === 401) window.location.href = '/api/auth/login';
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}

export async function deleteIdea(id: string, csrfToken: string): Promise<void> {
  const res = await fetch(`/api/ideas/${id}`, {
    method: 'DELETE',
    credentials: 'same-origin',
    headers: { 'x-csrf-token': csrfToken }
  });
  if (res.status === 401) window.location.href = '/api/auth/login';
  if (!res.ok) throw new Error(await res.text());
}

export async function setUpvote(id: string, upvoted: boolean, csrfToken: string): Promise<Idea> {
  const res = await fetch(`/api/ideas/${id}/upvote`, {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'content-type': 'application/json', 'x-csrf-token': csrfToken },
    body: JSON.stringify({ upvoted })
  });
  if (res.status === 401) window.location.href = '/api/auth/login';
  if (!res.ok) throw new Error(await res.text());
  const data = await res.json();
  return data.idea;
}

export async function logout(csrfToken: string): Promise<void> {
  await fetch('/api/auth/logout', {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'x-csrf-token': csrfToken }
  });
}
