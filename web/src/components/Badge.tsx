import type { ReactNode } from 'react';

export type BadgeTone = 'neutral' | 'success' | 'danger' | 'warning';

export interface BadgeProps {
  tone?: BadgeTone;
  children: ReactNode;
  /** Optional visually-hidden prefix, e.g. "Status: " for screen readers. */
  srPrefix?: string;
}

const TONE_CLASS: Record<BadgeTone, string> = {
  neutral: '',
  success: 'badge--success',
  danger: 'badge--danger',
  warning: 'badge--warning',
};

/** Small status pill. Color is paired with text so it is not the only cue. */
export function Badge({ tone = 'neutral', children, srPrefix }: BadgeProps) {
  return (
    <span className={['badge', TONE_CLASS[tone]].filter(Boolean).join(' ')}>
      {srPrefix ? <span className="visually-hidden">{srPrefix}</span> : null}
      {children}
    </span>
  );
}
