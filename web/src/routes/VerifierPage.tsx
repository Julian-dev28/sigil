import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Field } from '../components/Field';
import { Button } from '../components/Button';
import { Badge } from '../components/Badge';
import { CodeBlock } from '../components/CodeBlock';
import { CredentialCard } from '../components/CredentialCard';
import { useToast } from '../components/toast-context';
import { apiClient, ApiError } from '../api/client';
import type { VerifyCredentialResponse } from '../api/client';
import { truncateMiddle } from '../lib/format';

const CHECK_LABELS: Record<string, string> = {
  signature: 'Signature valid',
  notExpired: 'Not expired',
  notRevoked: 'Not revoked',
};

/** Verifier view: paste a JWT-VC and see per-check results. */
export function VerifierPage() {
  const toast = useToast();
  const [jwt, setJwt] = useState('');
  const [error, setError] = useState<string | undefined>(undefined);

  const mutation = useMutation<VerifyCredentialResponse, ApiError, string>({
    mutationFn: (token: string) => apiClient.verifyCredential({ jwt: token }),
    onError: (err) => {
      toast.push(err.message, 'error');
    },
  });

  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmed = jwt.trim();
    if (!trimmed) {
      setError('Paste a JWT-VC to verify.');
      return;
    }
    if (trimmed.split('.').length !== 3) {
      setError('That does not look like a compact JWT (expected three dot-separated parts).');
      return;
    }
    setError(undefined);
    mutation.mutate(trimmed);
  };

  const result = mutation.data;
  const checkEntries = result
    ? (Object.entries(result.checks) as [keyof typeof result.checks, boolean][])
    : [];

  return (
    <section aria-labelledby="verifier-heading">
      <header className="page-header">
        <h1 id="verifier-heading">Verify a credential</h1>
        <p>
          Paste a compact JWT-VC. Sigil checks the issuer signature, expiry, and
          StatusList2021 revocation status.
        </p>
      </header>

      <div className="stack">
        <form className="card stack" onSubmit={handleSubmit} noValidate>
          <Field
            as="textarea"
            label="JWT-VC"
            name="jwt"
            value={jwt}
            onChange={(e) => setJwt(e.target.value)}
            placeholder="eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCI…"
            hint="The compact, three-part JWT credential to verify."
            spellCheck={false}
            {...(error ? { error } : {})}
          />
          <div className="row">
            <Button type="submit" variant="primary" disabled={mutation.isPending}>
              {mutation.isPending ? 'Verifying…' : 'Verify credential'}
            </Button>
            {jwt ? (
              <Button
                type="button"
                variant="ghost"
                onClick={() => {
                  setJwt('');
                  setError(undefined);
                  mutation.reset();
                }}
              >
                Clear
              </Button>
            ) : null}
          </div>
        </form>

        <div aria-live="polite" className="stack">
          {mutation.isError && mutation.error ? (
            <p className="inline-error" role="alert">
              Verification failed: {mutation.error.message}
            </p>
          ) : null}

          {result ? (
            <div className="stack" data-testid="verify-result">
              <div
                className={`result-banner ${
                  result.valid ? 'result-banner--valid' : 'result-banner--invalid'
                }`}
                role="status"
              >
                <h2>{result.valid ? 'Credential is valid' : 'Credential is not valid'}</h2>
              </div>

              <ul className="check-list">
                {checkEntries.map(([key, passed]) => (
                  <li className="check-list__item" key={key}>
                    <span className="check-list__label">
                      {CHECK_LABELS[key] ?? key}
                    </span>
                    <Badge tone={passed ? 'success' : 'danger'} srPrefix="Result: ">
                      {passed ? 'Pass' : 'Fail'}
                    </Badge>
                  </li>
                ))}
              </ul>

              <dl className="cred-card__meta card">
                <dt>Issuer</dt>
                <dd className="mono" title={result.issuer}>
                  {truncateMiddle(result.issuer, 22, 12)}
                </dd>
                <dt>Subject</dt>
                <dd className="mono" title={result.subject}>
                  {truncateMiddle(result.subject, 22, 12)}
                </dd>
              </dl>

              <CredentialCard
                credential={result.credential}
                revoked={!result.checks.notRevoked}
              />

              <CodeBlock
                label="Credential JSON"
                value={JSON.stringify(result.credential, null, 2)}
                onCopied={() => toast.push('Credential JSON copied', 'success')}
              />
            </div>
          ) : null}
        </div>
      </div>
    </section>
  );
}

export default VerifierPage;
