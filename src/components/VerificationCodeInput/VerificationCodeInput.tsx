import { FormField } from "../FormField/FormField";

export function VerificationCodeInput({
  value,
  onChange,
  length,
}: {
  value: string;
  onChange: (value: string) => void;
  length: number | null;
}) {
  return (
    <FormField
      id="verification-code"
      label="Telegram verification code"
      autoComplete="one-time-code"
      inputMode={length === null ? "text" : "numeric"}
      pattern={length === null ? undefined : "[0-9]*"}
      maxLength={length ?? undefined}
      value={value}
      onChange={(event) => {
        onChange(length === null ? event.target.value : event.target.value.replace(/\D/g, ""));
      }}
      placeholder="Enter the code"
      required
    />
  );
}
