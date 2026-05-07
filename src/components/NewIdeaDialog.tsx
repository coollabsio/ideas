import * as React from 'react';
import { useState } from 'react';
import type { Idea } from '~/lib/github';
import { Button } from './ui/button';
import { Dialog } from './ui/dialog';

interface NewIdeaDialogProps {
  open: boolean;
  onClose: () => void;
}

const TITLE_MIN = 10;
const TITLE_MAX = 300;
const BODY_MIN = 30;
const BODY_MAX = 10000;

const inputClass =
  'w-full rounded-sm border-2 border-coolgray-300 bg-base px-2 py-1.5 text-sm text-white placeholder:text-neutral-600 outline-0 transition-colors focus:border-warning/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-warning focus-visible:ring-offset-2 focus-visible:ring-offset-coolgray-100 disabled:opacity-50';

export function NewIdeaDialog({ open, onClose }: NewIdeaDialogProps): React.ReactElement {
  const [title, setTitle] = useState('');
  const [body, setBody] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const titleLen = title.trim().length;
  const bodyLen = body.trim().length;
  const titleValid = titleLen >= TITLE_MIN && titleLen <= TITLE_MAX;
  const bodyValid = bodyLen >= BODY_MIN && bodyLen <= BODY_MAX;
  const canSubmit = titleValid && bodyValid && !submitting;

  function reset(): void {
    setTitle('');
    setBody('');
    setError(null);
    setSubmitting(false);
  }

  function handleClose(): void {
    if (submitting) return;
    reset();
    onClose();
  }

  async function handleSubmit(e: React.FormEvent<HTMLFormElement> | React.MouseEvent<HTMLButtonElement>): Promise<void> {
    e.preventDefault();
    if (!canSubmit) return;
    setSubmitting(true);
    setError(null);
    try {
      const csrfToken = window.__ideasCsrfToken ?? '';
      const res = await fetch('/api/create-idea', {
        method: 'POST',
        credentials: 'same-origin',
        headers: {
          'content-type': 'application/json',
          'x-csrf-token': csrfToken,
        },
        body: JSON.stringify({ title: title.trim(), body: body.trim() }),
      });
      if (res.status === 401) {
        setError('Session expired. Please sign in again.');
        return;
      }
      if (!res.ok) {
        const text = await res.text();
        setError(text || `HTTP ${res.status}`);
        return;
      }
      const idea = (await res.json()) as Idea;
      window.dispatchEvent(new CustomEvent('idea:created', { detail: idea }));
      reset();
      onClose();
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      title="New idea"
      description="Post a new idea to GitHub Discussions. Title and body are required."
    >
      <form onSubmit={(e) => void handleSubmit(e)} className="flex flex-col gap-3">
        <div>
          <div className="mb-1 flex items-center justify-between">
            <label htmlFor="new-idea-title" className="text-xs font-bold uppercase tracking-widest text-neutral-300">
              Title
            </label>
            <span
              className={`font-mono text-[10px] ${
                titleValid ? 'text-neutral-500' : 'text-warning'
              }`}
            >
              {titleLen}/{TITLE_MAX}
            </span>
          </div>
          <input
            id="new-idea-title"
            type="text"
            className={inputClass}
            placeholder="A short, descriptive title"
            value={title}
            maxLength={TITLE_MAX}
            disabled={submitting}
            onChange={(e) => setTitle(e.target.value)}
            required
          />
        </div>

        <div>
          <div className="mb-1 flex items-center justify-between">
            <label htmlFor="new-idea-body" className="text-xs font-bold uppercase tracking-widest text-neutral-300">
              Body
            </label>
            <span
              className={`font-mono text-[10px] ${
                bodyValid ? 'text-neutral-500' : 'text-warning'
              }`}
            >
              {bodyLen}/{BODY_MAX}
            </span>
          </div>
          <textarea
            id="new-idea-body"
            className={`${inputClass} min-h-40 resize-y font-sans`}
            placeholder="Describe the idea: what it does, why it's useful, any details that help others evaluate it. Markdown supported."
            value={body}
            maxLength={BODY_MAX}
            disabled={submitting}
            onChange={(e) => setBody(e.target.value)}
            rows={8}
            required
          />
          <p className="mt-1 text-[10px] text-neutral-600">
            Posted to GitHub Discussions in the “Ideas” category as your account.
          </p>
        </div>

        {error && (
          <div className="rounded-sm border border-error/50 bg-error/10 px-3 py-2 text-xs text-error">
            {error}
          </div>
        )}

        <div className="mt-1 flex items-center justify-end gap-2">
          <Button variant="ghost" onClick={handleClose} disabled={submitting}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="highlighted"
            disabled={!canSubmit}
            onClick={(e) => void handleSubmit(e)}
          >
            {submitting ? 'Posting…' : 'Post idea'}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
