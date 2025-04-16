import { useCallback, useState } from 'react';
import { Button } from './Button';

export interface CodeBlockProps {
  /** Raw text to display (already stringified). */
  value: string;
  /** Small label shown in the toolbar, e.g. "JWT" or "JSON". */
  label?: string;
  /** Enable a copy-to-clipboard button (default true). */
  copyable?: boolean;
  /** Called after a successful copy, e.g. to raise a toast. */
  onCopied?: () => void;
}

/** Read-only, horizontally scrollable code display with copy support. */
export function CodeBlock({
  value,
  label,
  copyable = true,
  onCopied,
}: CodeBlockProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(value);
      } else {
        throw new Error('Clipboard API unavailable');
      }
      setCopied(true);
      onCopied?.();
      window.setTimeout(() => setCopied(false), 1800);
    } catch {
      setCopied(false);
    }
  }, [value, onCopied]);

  return (
    <div className="codeblock">
      <div className="codeblock__bar">
        <span>{label ?? 'Text'}</span>
        {copyable ? (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleCopy}
            aria-label={`Copy ${label ?? 'text'} to clipboard`}
          >
            {copied ? 'Copied' : 'Copy'}
          </Button>
        ) : null}
      </div>
      <pre className="codeblock__pre" tabIndex={0}>
        <code>{value}</code>
      </pre>
    </div>
  );
}
