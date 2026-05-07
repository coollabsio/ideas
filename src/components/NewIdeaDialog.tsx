import * as React from 'react';
import { useState } from 'react';
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
const NEW_ISSUE_URL = 'https://github.com/coollabsio/ideas/issues/new';

const inputClass = (dirty: boolean): string =>
  `input ${dirty ? 'input-dirty' : ''}`;
const textareaClass = (dirty: boolean): string =>
  `input textarea min-h-40 resize-y ${dirty ? 'input-dirty' : ''}`;

export function NewIdeaDialog({ open, onClose }: NewIdeaDialogProps): React.ReactElement {
  const [title, setTitle] = useState('');
  const [body, setBody] = useState('');

  const titleLen = title.trim().length;
  const bodyLen = body.trim().length;
  const titleValid = titleLen >= TITLE_MIN && titleLen <= TITLE_MAX;
  const bodyValid = bodyLen >= BODY_MIN && bodyLen <= BODY_MAX;
  const canSubmit = titleValid && bodyValid;

  function reset(): void {
    setTitle('');
    setBody('');
  }

  function handleClose(): void {
    reset();
    onClose();
  }

  function handleSubmit(e: React.SyntheticEvent<HTMLFormElement>): void {
    e.preventDefault();
    if (!canSubmit) return;

    const params = new URLSearchParams({
      title: title.trim(),
      body: body.trim(),
      labels: 'idea',
    });

    window.location.href = `${NEW_ISSUE_URL}?${params.toString()}`;
  }

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      title="New idea"
      description="Draft your idea here, then finish posting it on GitHub."
    >
      <form onSubmit={handleSubmit} className="flex flex-col gap-3">
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
            onChange={(e) => setBody(e.target.value)}
            rows={8}
            required
          />
          <p className="mt-1 text-xs text-neutral-500 dark:text-neutral-600">
            You’ll continue on GitHub. Your GitHub account will be shown as the issue author.
          </p>
        </div>

        <div className="mt-1 flex items-center justify-end gap-2">
          <Button variant="ghost" onClick={handleClose}>
            Cancel
          </Button>
          <Button
            type="submit"
            variant="highlighted"
            disabled={!canSubmit}
          >
            Continue on GitHub
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
