import type { ReactNode } from 'react';
import type { VerifiableCredential } from '../api/client';
import { formatDate, primaryType, truncateMiddle } from '../lib/format';
import { Badge } from './Badge';

export interface CredentialCardProps {
  credential: VerifiableCredential;
  revoked?: boolean;
  /** Optional footer actions (buttons, links). */
  actions?: ReactNode;
}

/** Summary card for a single credential, used across Wallet and Issuer views. */
export function CredentialCard({
  credential,
  revoked,
  actions,
}: CredentialCardProps) {
  const type = primaryType(credential.type);
  const subject = credential.credentialSubject.id ?? '—';

  return (
    <article className="card cred-card" aria-label={`${type} credential`}>
      <div className="cred-card__head">
        <h3 className="cred-card__type">{type}</h3>
        {revoked ? (
          <Badge tone="danger" srPrefix="Status: ">
            Revoked
          </Badge>
        ) : (
          <Badge tone="success" srPrefix="Status: ">
            Active
          </Badge>
        )}
      </div>

      <dl className="cred-card__meta">
        <dt>Issuer</dt>
        <dd className="mono" title={credential.issuer}>
          {truncateMiddle(credential.issuer, 18, 10)}
        </dd>
        <dt>Subject</dt>
        <dd className="mono" title={subject}>
          {truncateMiddle(subject, 18, 10)}
        </dd>
        <dt>Issued</dt>
        <dd>{formatDate(credential.issuanceDate)}</dd>
        <dt>Expires</dt>
        <dd>{formatDate(credential.expirationDate)}</dd>
      </dl>

      {actions ? <div className="row">{actions}</div> : null}
    </article>
  );
}
