/** Match all whitespace-separated query terms in any order. */
export function matchesSearch(text: string, query: string): boolean {
  const normalize = (value: string) => value.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLocaleLowerCase();
  const haystack = normalize(text);
  return normalize(query).trim().split(/\s+/).every((term) => haystack.includes(term));
}
