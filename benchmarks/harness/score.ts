export function normalizeForMatch(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .replace(/\s+/g, " ")
    .trim();
}

export function tokenize(value: string): string[] {
  return normalizeForMatch(value).split(" ").filter(Boolean);
}

export interface TermMatch {
  term: string;
  found: boolean;
}

export function termMatches(text: string, terms: string[]): TermMatch[] {
  const haystack = normalizeForMatch(text);
  return terms.map((term) => ({ term, found: haystack.includes(normalizeForMatch(term)) }));
}

export function bannedTermHits(text: string, terms: string[]): string[] {
  const haystack = normalizeForMatch(text);
  return terms.filter((term) => haystack.includes(normalizeForMatch(term)));
}

export interface TokenStats {
  recall: number;
  precision: number;
  expectedCount: number;
  actualCount: number;
}

/**
 * Token-level recall/precision between an expected text and an extracted text.
 * Multiplicity-aware (bag of tokens): a repeated word that is missing or
 * duplicated counts each time, so engines are compared against the documented
 * word-accuracy metric rather than distinct tokens.
 */
export function tokenStats(expected: string, actual: string): TokenStats {
  const expectedTokens = tokenize(expected);
  const actualTokens = tokenize(actual);
  if (expectedTokens.length === 0) {
    return { recall: 1, precision: actualTokens.length === 0 ? 1 : 0, expectedCount: 0, actualCount: actualTokens.length };
  }

  const expectedCounts = new Map<string, number>();
  for (const token of expectedTokens) {
    expectedCounts.set(token, (expectedCounts.get(token) ?? 0) + 1);
  }
  const actualCounts = new Map<string, number>();
  for (const token of actualTokens) {
    actualCounts.set(token, (actualCounts.get(token) ?? 0) + 1);
  }

  let correct = 0;
  for (const [token, count] of expectedCounts) {
    correct += Math.min(count, actualCounts.get(token) ?? 0);
  }

  return {
    recall: correct / expectedTokens.length,
    precision: actualTokens.length === 0 ? 0 : correct / actualTokens.length,
    expectedCount: expectedTokens.length,
    actualCount: actualTokens.length,
  };
}

/** Simple string-similarity for title/author matching: 1 exact, 0.75 contained, 0 otherwise. */
export function stringScore(expected: string, actual: string): number {
  const want = normalizeForMatch(expected);
  const got = normalizeForMatch(actual);
  if (want === got) return 1;
  if (want.length > 0 && (got.includes(want) || want.includes(got))) return 0.75;
  return 0;
}

export function average(values: number[]): number {
  if (values.length === 0) return 1;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}
