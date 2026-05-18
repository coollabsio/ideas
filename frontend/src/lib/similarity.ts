import type { Idea } from '$lib/api';

export interface SimilarIdeaMatch {
  idea: Idea;
  score: number;
}

interface SimilarityOptions {
  limit?: number;
  minScore?: number;
}

const STOP_WORDS = new Set([
  'a',
  'an',
  'and',
  'are',
  'as',
  'at',
  'be',
  'by',
  'can',
  'for',
  'from',
  'have',
  'how',
  'i',
  'in',
  'is',
  'it',
  'its',
  'of',
  'on',
  'or',
  'our',
  'that',
  'the',
  'this',
  'to',
  'we',
  'with',
  'you',
  'your'
]);

const DEFAULT_LIMIT = 3;
const DEFAULT_MIN_SCORE = 0.36;
const MIN_DRAFT_TITLE_LENGTH = 6;
const MIN_DRAFT_BODY_LENGTH = 20;

export function findSimilarIdeas(title: string, body: string, ideas: Idea[], options: SimilarityOptions = {}): SimilarIdeaMatch[] {
  const draftTitle = title.trim();
  const draftBody = body.trim();

  if (draftTitle.length < MIN_DRAFT_TITLE_LENGTH && draftBody.length < MIN_DRAFT_BODY_LENGTH) {
    return [];
  }

  const limit = options.limit ?? DEFAULT_LIMIT;
  const minScore = options.minScore ?? DEFAULT_MIN_SCORE;
  const draftTitleTokens = tokenize(draftTitle);
  const draftAllTokens = tokenize(`${draftTitle} ${draftBody}`);
  const draftNormalizedTitle = normalize(draftTitle);

  if (draftAllTokens.size === 0) return [];

  return ideas
    .map((idea) => ({ idea, score: scoreIdea({ draftTitleTokens, draftAllTokens, draftNormalizedTitle }, idea) }))
    .filter((match) => match.score >= minScore)
    .sort((a, b) => b.score - a.score || b.idea.upvoteCount - a.idea.upvoteCount || b.idea.createdAt.localeCompare(a.idea.createdAt))
    .slice(0, limit);
}

function scoreIdea(
  draft: { draftTitleTokens: Set<string>; draftAllTokens: Set<string>; draftNormalizedTitle: string },
  idea: Idea
): number {
  const ideaTitleTokens = tokenize(idea.title);
  const ideaAllTokens = tokenize(`${idea.title} ${idea.bodyText}`);
  const ideaNormalizedTitle = normalize(idea.title);

  const titleScore = overlapScore(draft.draftTitleTokens, ideaTitleTokens);
  const allScore = overlapScore(draft.draftAllTokens, ideaAllTokens);
  const phraseBonus = phraseSimilarityBonus(draft.draftNormalizedTitle, ideaNormalizedTitle);
  const statusBonus = idea.status === 'closed' ? -0.06 : 0;

  return titleScore * 0.62 + allScore * 0.32 + phraseBonus + statusBonus;
}

function phraseSimilarityBonus(a: string, b: string): number {
  if (!a || !b) return 0;
  if (a === b) return 0.3;
  if (a.length >= 10 && b.includes(a)) return 0.18;
  if (b.length >= 10 && a.includes(b)) return 0.18;
  return 0;
}

function overlapScore(a: Set<string>, b: Set<string>): number {
  if (a.size === 0 || b.size === 0) return 0;

  let overlap = 0;
  for (const token of a) {
    if (b.has(token)) overlap += 1;
  }

  if (overlap === 0) return 0;

  const precision = overlap / a.size;
  const recall = overlap / b.size;
  return (2 * precision * recall) / (precision + recall);
}

function tokenize(value: string): Set<string> {
  return new Set(
    normalize(value)
      .split(' ')
      .map(stemToken)
      .filter((token) => token.length >= 3 && !STOP_WORDS.has(token))
  );
}

function stemToken(token: string): string {
  return token.replace(/(?:ing|ers|er|ed|es|s)$/u, '');
}

function normalize(value: string): string {
  return value
    .toLowerCase()
    .normalize('NFKD')
    .replace(/[\u0300-\u036f]/gu, '')
    .replace(/[^a-z0-9]+/gu, ' ')
    .trim()
    .replace(/\s+/gu, ' ');
}
