/** Small display helpers shared across routes. */

/** Format an ISO date string for display; falls back to the raw value. */
export function formatDate(iso: string | undefined): string {
  if (!iso) return '—';
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/** Return the most specific credential type (drops the base `VerifiableCredential`). */
export function primaryType(types: string[]): string {
  const specific = types.find((t) => t !== 'VerifiableCredential');
  return specific ?? types[0] ?? 'VerifiableCredential';
}

/** Truncate a long identifier (DID / JWT) for compact display. */
export function truncateMiddle(value: string, head = 12, tail = 8): string {
  if (value.length <= head + tail + 1) return value;
  return `${value.slice(0, head)}…${value.slice(-tail)}`;
}
