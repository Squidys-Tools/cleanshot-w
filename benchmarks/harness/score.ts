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
  const haystack = text.toLowerCase();
  return terms.map((term) => ({ term, found: haystack.includes(term.toLowerCase()) }));
}

export function bannedTermHits(text: string, terms: string[]): string[] {
  const haystack = text.toLowerCase();
  return terms.filter((term) => haystack.includes(term.toLowerCase()));
}

export interface TokenStats {
  recall: number;
  precision: number;
  expectedCount: number;
  actualCount: number;
}

/** Token-level recall/precision between an expected text and an extracted text. */
export function tokenStats(expected: string, actual: string): TokenStats {
  const expectedTokens = [...new Set(tokenize(expected))];
  const actualTokens = new Set(tokenize(actual));
  if (expectedTokens.length === 0) {
    return { recall: 1, precision: actualTokens.size === 0 ? 1 : 0, expectedCount: 0, actualCount: actualTokens.size };
  }
  const hits = expectedTokens.filter((token) => actualTokens.has(token)).length;
  return {
    recall: hits / expectedTokens.length,
    precision: actualTokens.size === 0 ? 0 : hits / actualTokens.size,
    expectedCount: expectedTokens.length,
    actualCount: actualTokens.size,
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
