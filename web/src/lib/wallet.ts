/**
 * Holder wallet storage.
 *
 * The wallet keeps issued/received credentials in `localStorage` so the holder
 * owns their credentials client-side. Each entry stores the compact JWT plus the
 * decoded credential object for display.
 */

import type { VerifiableCredential } from '../api/client';

export interface WalletEntry {
  /** Credential id (`urn:uuid:...`). */
  id: string;
  /** Compact JWT-VC. */
  jwt: string;
  /** Decoded credential object. */
  credential: VerifiableCredential;
  /** When this entry was added to the wallet (ISO 8601). */
  savedAt: string;
}

const STORAGE_KEY = 'sigil.wallet.v1';

/** Event dispatched whenever the wallet changes, for cross-component sync. */
export const WALLET_EVENT = 'sigil:wallet-changed';

function hasStorage(): boolean {
  try {
    return typeof window !== 'undefined' && !!window.localStorage;
  } catch {
    return false;
  }
}

function isWalletEntry(value: unknown): value is WalletEntry {
  if (typeof value !== 'object' || value === null) return false;
  const v = value as Record<string, unknown>;
  return (
    typeof v.id === 'string' &&
    typeof v.jwt === 'string' &&
    typeof v.savedAt === 'string' &&
    typeof v.credential === 'object' &&
    v.credential !== null
  );
}

/** Read all wallet entries, newest first. Never throws. */
export function loadWallet(): WalletEntry[] {
  if (!hasStorage()) return [];
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isWalletEntry);
  } catch {
    return [];
  }
}

function persist(entries: WalletEntry[]): void {
  if (!hasStorage()) return;
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(entries));
  window.dispatchEvent(new CustomEvent(WALLET_EVENT));
}

/** Add or replace a credential in the wallet (keyed by id). Returns new list. */
export function saveToWallet(
  entry: Omit<WalletEntry, 'savedAt'> & { savedAt?: string },
): WalletEntry[] {
  const existing = loadWallet().filter((e) => e.id !== entry.id);
  const next: WalletEntry[] = [
    { ...entry, savedAt: entry.savedAt ?? new Date().toISOString() },
    ...existing,
  ];
  persist(next);
  return next;
}

/** Remove a credential from the wallet by id. Returns new list. */
export function removeFromWallet(id: string): WalletEntry[] {
  const next = loadWallet().filter((e) => e.id !== id);
  persist(next);
  return next;
}

/** Remove every credential from the wallet. */
export function clearWallet(): void {
  persist([]);
}
