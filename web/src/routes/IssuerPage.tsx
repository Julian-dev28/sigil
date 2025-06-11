import { useState } from 'react';
import { useMutation } from '@tanstack/react-query';
import { Field } from '../components/Field';
import { Button } from '../components/Button';
import { CredentialCard } from '../components/CredentialCard';
import { CodeBlock } from '../components/CodeBlock';
import { useToast } from '../components/toast-context';
import { apiClient, ApiError } from '../api/client';
import type { IssueCredentialResponse } from '../api/client';
import { saveToWallet } from '../lib/wallet';

interface FormErrors {
  subjectId?: string;
  type?: string;
  claims?: string;
  expiresInDays?: string;
}

const DID_KEY_RE = /^did:(key|web):.+/;

function validate(
  subjectId: string,
  type: string,
  claimsText: string,
  expiresInDays: string,
): {
  errors: FormErrors;
  claims: Record<string, unknown> | undefined;
  expiry: number | undefined;
} {
  const errors: FormErrors = {};

  if (!subjectId.trim()) {
    errors.subjectId = 'Subject DID is required.';
  } else if (!DID_KEY_RE.test(subjectId.trim())) {
    errors.subjectId = 'Enter a valid DID, e.g. did:key:z6Mk… or did:web:…';
  }

  if (!type.trim()) {
    errors.type = 'Credential type is required.';
  } else if (!/^[A-Za-z][A-Za-z0-9]*$/.test(type.trim())) {
    errors.type = 'Use a single PascalCase type name, e.g. EmploymentCredential.';
  }

  let claims: Record<string, unknown> | undefined;
  const trimmedClaims = claimsText.trim();
  if (!trimmedClaims) {
    claims = {};
  } else {
    try {
      const parsed: unknown = JSON.parse(trimmedClaims);
      if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
        errors.claims = 'Claims must be a JSON object.';
      } else {
        claims = parsed as Record<string, unknown>;
      }
    } catch (err) {
      errors.claims = `Invalid JSON: ${err instanceof Error ? err.message : 'parse error'}`;
    }
  }

  let expiry: number | undefined;
  const trimmedExpiry = expiresInDays.trim();
  if (trimmedExpiry) {
    const n = Number(trimmedExpiry);
    if (!Number.isInteger(n) || n <= 0) {
      errors.expiresInDays = 'Expiry must be a positive whole number of days.';
    } else {
      expiry = n;
    }
  }

  return { errors, claims, expiry };
}

/** Issuer view: build and sign a credential via the server. */
export function IssuerPage() {
  const toast = useToast();
  const [subjectId, setSubjectId] = useState('');
  const [type, setType] = useState('EmploymentCredential');
  const [claimsText, setClaimsText] = useState(
    '{\n  "role": "Software Engineer",\n  "employer": "Acme Corp"\n}',
  );
  const [expiresInDays, setExpiresInDays] = useState('365');
  const [errors, setErrors] = useState<FormErrors>({});

  const mutation = useMutation<
    IssueCredentialResponse,
    ApiError,
    {
      subjectId: string;
      type: string;
      claims: Record<string, unknown>;
      expiresInDays?: number;
    }
  >({
    mutationFn: (variables) => apiClient.issueCredential(variables),
    onSuccess: (data) => {
      saveToWallet({ id: data.id, jwt: data.jwt, credential: data.credential });
      toast.push('Credential issued and added to your wallet', 'success');
    },
    onError: (err) => {
      toast.push(err.message, 'error');
    },
  });

  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const { errors: nextErrors, claims, expiry } = validate(
      subjectId,
      type,
      claimsText,
      expiresInDays,
    );
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length > 0 || !claims) return;

    mutation.mutate({
      subjectId: subjectId.trim(),
      type: type.trim(),
      claims,
      ...(expiry !== undefined ? { expiresInDays: expiry } : {}),
    });
  };

  const result = mutation.data;

  return (
    <section aria-labelledby="issuer-heading">
      <header className="page-header">
        <h1 id="issuer-heading">Issue a credential</h1>
        <p>
          Sign a new W3C Verifiable Credential with the issuer key. The signed
          JWT-VC is added to your wallet automatically.
        </p>
      </header>

      <div className="stack">
        <form className="card stack" onSubmit={handleSubmit} noValidate>
          <Field
            label="Subject DID"
            name="subjectId"
            value={subjectId}
            onChange={(e) => setSubjectId(e.target.value)}
            placeholder="did:key:z6Mk…"
            hint="The holder identifier the credential is issued to."
            autoComplete="off"
            spellCheck={false}
            {...(errors.subjectId ? { error: errors.subjectId } : {})}
          />
          <Field
            label="Credential type"
            name="type"
            value={type}
            onChange={(e) => setType(e.target.value)}
            placeholder="EmploymentCredential"
            hint="A single PascalCase type added alongside VerifiableCredential."
            autoComplete="off"
            spellCheck={false}
            {...(errors.type ? { error: errors.type } : {})}
          />
          <Field
            as="textarea"
            label="Claims (JSON)"
            name="claims"
            value={claimsText}
            onChange={(e) => setClaimsText(e.target.value)}
            hint="A JSON object of subject claims. Leave as {} for none."
            spellCheck={false}
            {...(errors.claims ? { error: errors.claims } : {})}
          />
          <Field
            label="Expires in (days)"
            name="expiresInDays"
            type="number"
            inputMode="numeric"
            min={1}
            value={expiresInDays}
            onChange={(e) => setExpiresInDays(e.target.value)}
            hint="Optional. Leave blank for a non-expiring credential."
            {...(errors.expiresInDays ? { error: errors.expiresInDays } : {})}
          />
          <div className="row">
            <Button type="submit" variant="primary" disabled={mutation.isPending}>
              {mutation.isPending ? 'Issuing…' : 'Issue credential'}
            </Button>
          </div>
        </form>

        <div aria-live="polite" className="stack">
          {mutation.isError && mutation.error ? (
            <p className="inline-error" role="alert">
              Could not issue credential: {mutation.error.message}
            </p>
          ) : null}

          {result ? (
            <div className="stack">
              <h2>Issued credential</h2>
              <CredentialCard credential={result.credential} revoked={false} />
              <CodeBlock
                label="JWT-VC"
                value={result.jwt}
                onCopied={() => toast.push('JWT copied to clipboard', 'success')}
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

export default IssuerPage;
