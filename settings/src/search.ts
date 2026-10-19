/** Match all whitespace-separated query terms in any order. */
export function matchesSearch(text: string, query: string): boolean {
  const haystack = text.toLocaleLowerCase();
  return query.trim().toLocaleLowerCase().split(/\s+/).every((term) => haystack.includes(term));
}
