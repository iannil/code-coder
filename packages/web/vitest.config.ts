import { defineConfig } from "vitest/config"
import react from "@vitejs/plugin-react"
import { resolve } from "path"

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}", "test/unit/**/*.test.{ts,tsx}"],
    exclude: ["node_modules", "dist", "test/e2e"],
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "json-summary", "html"],
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "src/vite-env.d.ts",
        "src/main.tsx",
        "src/**/*.test.{ts,tsx}",
        "src/components/ui/**", // UI primitives don't need unit tests
        "src/components/**/*.tsx", // Component tests need React Testing Library setup
        "src/pages/**/*.tsx", // Page tests need full app context
        "src/router.ts", // Router config
        "src/App.tsx", // App wrapper
        "src/**/index.ts", // Re-exports
      ],
      // Thresholds for core business logic files
      // Start with achievable goals and increase over time
      thresholds: {
        lines: 60,
        functions: 55,
        branches: 60,
        statements: 60,
      },
    },
  },
  resolve: {
    alias: {
      "@": resolve(__dirname, "./src"),
    },
  },
})
