import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { CredentialCard } from '../components/CredentialCard';
import { CodeBlock } from '../components/CodeBlock';
import { Button } from '../components/Button';
import { useToast } from '../components/toast-context';
import {
  loadWallet,
  removeFromWallet,
  WALLET_EVENT,
  type WalletEntry,
} from '../lib/wallet';
import { primaryType } from '../lib/format';

/** Holder view: credentials stored locally in the browser wallet. */
export function WalletPage() {
  const [entries, setEntries] = useState<WalletEntry[]>(() => loadWallet());
  const [expanded, setExpanded] = useState<string | null>(null);
  const toast = useToast();

  useEffect(() => {
    const sync = () => setEntries(loadWallet());
    window.addEventListener(WALLET_EVENT, sync);
    window.addEventListener('storage', sync);
    return () => {
      window.removeEventListener(WALLET_EVENT, sync);
      window.removeEventListener('storage', sync);
    };
  }, []);

  const handleRemove = useCallback(
    (id: string) => {
      setEntries(removeFromWallet(id));
      if (expanded === id) setExpanded(null);
      toast.push('Credential removed from wallet', 'info');
    },
    [expanded, toast],
  );

  return (
    <section aria-labelledby="wallet-heading">
      <header className="page-header">
        <h1 id="wallet-heading">Wallet</h1>
        <p>
          Credentials you hold are stored locally in this browser. Issue a new
          credential on the <Link to="/issuer">Issuer</Link> page or verify one on
          the <Link to="/verifier">Verifier</Link> page.
        </p>
      </header>

      {entries.length === 0 ? (
        <div className="empty-state">
          <p>Your wallet is empty.</p>
          <p>
            <Link to="/issuer">Issue your first credential</Link> to see it here.
          </p>
        </div>
      ) : (
        <ul className="grid-cards" style={{ listStyle: 'none', margin: 0, padding: 0 }}>
          {entries.map((entry) => {
            const isOpen = expanded === entry.id;
            const panelId = `wallet-detail-${entry.id}`;
            return (
              <li key={entry.id}>
                <CredentialCard
                  credential={entry.credential}
                  revoked={false}
                  actions={
                    <>
                      <Button
                        size="sm"
                        aria-expanded={isOpen}
                        aria-controls={panelId}
                        onClick={() => setExpanded(isOpen ? null : entry.id)}
                      >
                        {isOpen ? 'Hide details' : 'View details'}
                      </Button>
                      <Button
                        size="sm"
                        variant="danger"
                        onClick={() => handleRemove(entry.id)}
                        aria-label={`Remove ${primaryType(entry.credential.type)} credential`}
                      >
                        Remove
                      </Button>
                    </>
                  }
                />
                {isOpen ? (
                  <div id={panelId} className="stack" style={{ marginTop: 'var(--space-3)' }}>
                    <CodeBlock
                      label="JWT-VC"
                      value={entry.jwt}
                      onCopied={() => toast.push('JWT copied to clipboard', 'success')}
                    />
                    <CodeBlock
                      label="Credential JSON"
                      value={JSON.stringify(entry.credential, null, 2)}
                      onCopied={() => toast.push('Credential JSON copied', 'success')}
                    />
                  </div>
                ) : null}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

export default WalletPage;
