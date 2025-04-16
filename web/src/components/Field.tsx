import { useId } from 'react';
import type {
  InputHTMLAttributes,
  TextareaHTMLAttributes,
  ReactNode,
} from 'react';

interface BaseFieldProps {
  label: ReactNode;
  hint?: ReactNode;
  error?: string;
}

type InputFieldProps = BaseFieldProps &
  Omit<InputHTMLAttributes<HTMLInputElement>, 'id'> & {
    as?: 'input';
  };

type TextareaFieldProps = BaseFieldProps &
  Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, 'id'> & {
    as: 'textarea';
  };

export type FieldProps = InputFieldProps | TextareaFieldProps;

/**
 * A labelled form control with hint + error text, wired up for screen readers
 * via `aria-describedby` and `aria-invalid`.
 */
export function Field(props: FieldProps) {
  const generatedId = useId();
  const { label, hint, error, as = 'input', ...control } = props as FieldProps & {
    as?: 'input' | 'textarea';
  };

  const controlId = (control as { name?: string }).name
    ? `field-${(control as { name?: string }).name}`
    : generatedId;
  const hintId = hint ? `${controlId}-hint` : undefined;
  const errorId = error ? `${controlId}-error` : undefined;
  const describedBy = [hintId, errorId].filter(Boolean).join(' ') || undefined;

  const sharedProps = {
    id: controlId,
    className: 'field__control',
    'aria-invalid': error ? true : undefined,
    'aria-describedby': describedBy,
  };

  return (
    <div className="field">
      <label className="field__label" htmlFor={controlId}>
        {label}
      </label>
      {hint ? (
        <span className="field__hint" id={hintId}>
          {hint}
        </span>
      ) : null}
      {as === 'textarea' ? (
        <textarea
          {...sharedProps}
          {...(control as TextareaHTMLAttributes<HTMLTextAreaElement>)}
        />
      ) : (
        <input
          {...sharedProps}
          {...(control as InputHTMLAttributes<HTMLInputElement>)}
        />
      )}
      {error ? (
        <span className="field__error" id={errorId} role="alert">
          {error}
        </span>
      ) : null}
    </div>
  );
}
