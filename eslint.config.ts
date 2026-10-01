import eslint from "@eslint/js";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import globals from "globals";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: ["dist", "node_modules", "src-tauri/target"],
  },
  eslint.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  {
    linterOptions: { reportUnusedDisableDirectives: "error" },
    rules: {
      "complexity": ["error", { max: 20, variant: "modified" }],
      "max-depth": ["error", 3],
      "max-params": ["error", 4],
      "no-else-return": ["error", { allowElseIf: false }],
      "no-nested-ternary": "error",
      "no-lonely-if": "error",
      "eqeqeq": ["error", "always"],
      "curly": ["error", "multi-line"],
    },
  },
  {
    files: ["scripts/**/*.mjs"],
    ...tseslint.configs.disableTypeChecked,
    languageOptions: { globals: globals.node },
  },
  { files: ["eslint.config.ts"], rules: { "@typescript-eslint/no-deprecated": "off" } },
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2020,
      parserOptions: { project: ["./tsconfig.json", "./tsconfig.node.json"] },
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
    },
    rules: {
      "react-hooks/exhaustive-deps": "error",
      "react-hooks/rules-of-hooks": "error",
      "react-refresh/only-export-components": ["error", { allowConstantExport: true }],
    },
  },
);
