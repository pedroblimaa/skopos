import { useLanguage } from "../../i18n/useLanguage";
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
  const { t } = useLanguage();

  return (
    <FormField
      id="verification-code"
      label={t("verificationCode")}
      autoComplete="one-time-code"
      inputMode={length === null ? "text" : "numeric"}
      pattern={length === null ? undefined : "[0-9]*"}
      maxLength={length ?? undefined}
      value={value}
      onChange={(event) => {
        onChange(length === null ? event.target.value : event.target.value.replace(/\D/g, ""));
      }}
      placeholder={t("enterCode")}
      required
    />
  );
}
