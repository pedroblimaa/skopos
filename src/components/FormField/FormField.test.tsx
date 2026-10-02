import { cleanup, fireEvent, screen } from "@testing-library/react";
import { afterEach, expect, it, vi } from "vitest";
import { render } from "../../test-setup";
import { FormField } from "./FormField";

afterEach(cleanup);

it.each([
  ["en", "", "Fill out this field."],
  ["pt-BR", "", "Preencha este campo."],
  ["en", "invalid-email", "Enter a valid value."],
  ["pt-BR", "invalid-email", "Digite um valor válido."],
])("localizes %s validation for %s and clears it when typing", (language, value, expected) => {
  localStorage.setItem("skopos.language", language);
  const onInvalid = vi.fn();
  const onInput = vi.fn();
  render(
    <FormField
      id="email"
      label="Email"
      type="email"
      required
      defaultValue={value}
      onInvalid={onInvalid}
      onInput={onInput}
    />,
  );
  const input = screen.getByLabelText<HTMLInputElement>("Email");

  fireEvent.invalid(input);

  expect(input.validationMessage).toBe(expected);
  expect(onInvalid).toHaveBeenCalledTimes(1);

  fireEvent.input(input, { target: { value: "name@example.com" } });

  expect(input.validationMessage).toBe("");
  expect(onInput).toHaveBeenCalledTimes(1);
});
