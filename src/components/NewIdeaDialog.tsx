import * as React from 'react';
import { useState } from 'react';
import { AlertTriangle, Loader2 } from 'lucide-react';
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

const inputClass = (dirty: boolean): string =>
  `input ${dirty ? 'input-dirty' : ''}`;
const textareaClass = (dirty: boolean): string =>
  `input textarea min-h-40 resize-y ${dirty ? 'input-dirty' : ''}`;

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

  async function handleSubmit(e: React.SyntheticEvent<HTMLFormElement | HTMLButtonElement>): Promise<void> {
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
      description="Post a new idea as a GitHub Issue. Title and body are required."
    >
      <form onSubmit={(e) => void handleSubmit(e)} className="flex flex-col gap-3">
        <div>
          <div className="mb-1 flex items-center justify-between">
            <label htmlFor="new-idea-title" className="text-xs font-bold uppercase tracking-widest text-neutral-700 dark:text-neutral-300">
              Title
            </label>
            <span
              className={`font-mono text-xs ${
                titleValid ? 'text-neutral-500' : 'text-coollabs dark:text-warning'
              }`}
            >
              {titleLen}/{TITLE_MAX}
            </span>
          </div>
          <input
            id="new-idea-title"
            type="text"
            className={inputClass(title.length > 0)}
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
            <label htmlFor="new-idea-body" className="text-xs font-bold uppercase tracking-widest text-neutral-700 dark:text-neutral-300">
              Body
            </label>
            <span
              className={`font-mono text-xs ${
                bodyValid ? 'text-neutral-500' : 'text-coollabs dark:text-warning'
              }`}
            >
              {bodyLen}/{BODY_MAX}
            </span>
          </div>
          <textarea
            id="new-idea-body"
            className={textareaClass(body.length > 0)}
            placeholder="Describe the idea: what it does, why it's useful, any details that help others evaluate it. Markdown supported."
            value={body}
            maxLength={BODY_MAX}
            disabled={submitting}
            onChange={(e) => setBody(e.target.value)}
            rows={8}
            required
          />
          <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-600">
            Posted to GitHub Issues with the “idea” label as your account.
          </p>
        </div>

        {error && (
          <div className="callout callout-danger flex items-start gap-2 text-xs">
            <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-red-600 dark:text-red-400" aria-hidden="true" />
            <span>{error}</span>
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
            {submitting ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
                <span>Posting…</span>
              </>
            ) : (
              'Post idea'
            )}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
