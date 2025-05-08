import js from "@eslint/js";
import solid from "eslint-plugin-solid/configs/typescript";
import eslintPluginPrettierRecommended from "eslint-plugin-prettier/recommended";

import * as tsParser from "@typescript-eslint/parser";

export default [
  js.configs.recommended,
  eslintPluginPrettierRecommended,
  {
    files: ["**/*.{ts,tsx}"],
    ...solid,
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        project: "tsconfig.json",
      },
    },
    rules: {
      "prettier/prettier": ["warn"],
      "no-console": [
        "warn",
        {
          allow: ["info", "error"],
        },
      ],
      "no-undef": "off",
      "@typescript-eslint/ban-ts-comment": "off",
    },
  },
  {
    ignores: [
      "**/node_modules/",
      "**/dist/",
      "**/.output/",
      "**/.nuxt/",
      "**/.nitro/",
    ],
  },
];
