/**
 * Technology Fingerprint Database
 *
 * Comprehensive fingerprint patterns for detecting web technologies from HTML/JavaScript.
 * Each entry includes detection patterns with associated confidence levels.
 */

export interface FingerprintPattern {
  pattern: string | RegExp | string[]
  attribute?: string
  confidence: "high" | "medium" | "low"
  notes?: string
}

export interface TechnologyFingerprint {
  name: string
  patterns: FingerprintPattern[]
  category: string
  website?: string
}

export const FINGERPRINTS: Record<string, TechnologyFingerprint[]> = {
  // ============================================
  // FRONTEND FRAMEWORKS
  // ============================================
  frontend: [
    {
      name: "React",
      category: "framework",
      patterns: [
        { pattern: "__REACT__", confidence: "high" },
        { pattern: "react-root", confidence: "high" },
        { pattern: "data-reactroot", confidence: "high" },
        { pattern: "data-reactid", confidence: "high" },
        { pattern: ["_react", "ReactDOM"], confidence: "high" },
        { pattern: "React.createElement", confidence: "high" },
        { pattern: /@material-ui|@mui\/material/, confidence: "medium", notes: "React ecosystem" },
      ],
    },
    {
      name: "Vue",
      category: "framework",
      patterns: [
        { pattern: "__vue__", confidence: "high" },
        { pattern: "__VUE__", confidence: "high", notes: "Vue 3" },
        { pattern: "Vue.createApp", confidence: "high" },
        { pattern: "new Vue(", confidence: "high" },
        { pattern: "v-cloak", attribute: "style", confidence: "medium" },
        { pattern: "data-v-", attribute: "class", confidence: "medium" },
        { pattern: "vue-router", confidence: "medium" },
        { pattern: "pinia", confidence: "medium" },
      ],
    },
    {
      name: "Svelte",
      category: "framework",
      patterns: [
        { pattern: "data-svelte-h", attribute: "id", confidence: "high" },
        { pattern: "__svelte_component", confidence: "high" },
      ],
    },
    {
      name: "Angular",
      category: "framework",
      patterns: [
        { pattern: "ng-version", attribute: "ng-version", confidence: "high" },
        { pattern: "ng-app", confidence: "high" },
        { pattern: "ng-", attribute: "class", confidence: "medium" },
        { pattern: "*ngIf", confidence: "high" },
        { pattern: "*ngFor", confidence: "high" },
        { pattern: "[(ngModel)]", confidence: "high" },
        { pattern: "zone.js", confidence: "medium" },
        { pattern: "@angular/", confidence: "high" },
      ],
    },
    {
      name: "Solid",
      category: "framework",
      patterns: [
        { pattern: "data-hk", attribute: "id", confidence: "high" },
        { pattern: "createSignal", confidence: "high" },
        { pattern: "createEffect", confidence: "high" },
        { pattern: "@solidjs/", confidence: "high" },
      ],
    },
    {
      name: "Next.js",
      category: "framework",
      patterns: [
        { pattern: "__NEXT_DATA__", confidence: "high" },
        { pattern: "__NEXT_ROUTER__", confidence: "high" },
        { pattern: "/_next/static/", confidence: "high" },
        { pattern: "/_next/image", confidence: "high" },
        { pattern: "next/router", confidence: "medium" },
        { pattern: "next/link", confidence: "medium" },
        { pattern: "next/font", confidence: "medium" },
        { pattern: "next.config", confidence: "low" },
      ],
    },
    {
      name: "Nuxt",
      category: "framework",
      patterns: [
        { pattern: "__NUXT__", confidence: "high" },
        { pattern: "__nuxt__", confidence: "high" },
        { pattern: "/_nuxt/", confidence: "high" },
        { pattern: "nuxt-link", confidence: "medium" },
        { pattern: "@nuxt/", confidence: "high" },
      ],
    },
    {
      name: "Remix",
      category: "framework",
      patterns: [
        { pattern: "__remixContext", confidence: "high" },
        { pattern: "__remixRouteModules", confidence: "high" },
        { pattern: "/build/", confidence: "medium" },
        { pattern: "@remix-run/", confidence: "high" },
      ],
    },
    {
      name: "Astro",
      category: "framework",
      patterns: [
        { pattern: "astro-head", confidence: "medium" },
        { pattern: "astro-", attribute: "class", confidence: "medium" },
        { pattern: "@astrojs/", confidence: "high" },
      ],
    },
    {
      name: "Umi",
      category: "framework",
      patterns: [
        { pattern: "__UMI__", confidence: "high" },
        { pattern: "@umijs/", confidence: "high" },
        { pattern: "/umi.css", confidence: "medium" },
      ],
    },
    {
      name: "SvelteKit",
      category: "framework",
      patterns: [
        { pattern: "__sveltekit", confidence: "high" },
        { pattern: "/_app/immutable/", confidence: "high" },
        { pattern: "@sveltejs/kit", confidence: "high" },
      ],
    },
    {
      name: "Gatsby",
      category: "framework",
      patterns: [
        { pattern: "gatsby", confidence: "medium" },
        { pattern: "/page-data/", confidence: "high" },
        { pattern: "@gatsbyjs/", confidence: "high" },
      ],
    },
    {
      name: "Qwik",
      category: "framework",
      patterns: [
        { pattern: "qwik-", attribute: "class", confidence: "high" },
        { pattern: "@builder.io/qwik", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // UI LIBRARIES
  // ============================================
  ui: [
    {
      name: "Tailwind CSS",
      category: "ui",
      patterns: [
        { pattern: /flex|grid|p-\[|px-\[|py-\[/g, confidence: "low", notes: "Common utilities, verify with more specific patterns" },
        { pattern: "tailwindcss", confidence: "high" },
        { pattern: "@tailwind", confidence: "high" },
        { pattern: "tw-", confidence: "medium" },
        { pattern: /[a-z]-\[[0-9]/, confidence: "medium", notes: "Arbitrary values" },
      ],
    },
    {
      name: "Material-UI (MUI)",
      category: "ui",
      patterns: [
        { pattern: "makeStyles", confidence: "high" },
        { pattern: "MuiBox-", attribute: "class", confidence: "high" },
        { pattern: "MuiButton-", attribute: "class", confidence: "high" },
        { pattern: "MuiGrid-", attribute: "class", confidence: "high" },
        { pattern: "MuiPaper-", attribute: "class", confidence: "high" },
        { pattern: "@mui/material", confidence: "high" },
        { pattern: "@emotion/react", confidence: "medium", notes: "MUI v5 uses emotion" },
        { pattern: "@mui/icons-material", confidence: "high" },
      ],
    },
    {
      name: "Ant Design",
      category: "ui",
      patterns: [
        { pattern: "ant-", attribute: "class", confidence: "high" },
        { pattern: "ant-btn", confidence: "high" },
        { pattern: "ant-input", confidence: "high" },
        { pattern: "ant-modal", confidence: "high" },
        { pattern: "antd", confidence: "high" },
        { pattern: "@ant-design/", confidence: "high" },
      ],
    },
    {
      name: "Chakra UI",
      category: "ui",
      patterns: [
        { pattern: "chakra-", attribute: "class", confidence: "high" },
        { pattern: "css-", attribute: "class", confidence: "medium" },
        { pattern: "@chakra-ui/react", confidence: "high" },
        { pattern: "useTheme", confidence: "low" },
      ],
    },
    {
      name: "shadcn/ui",
      category: "ui",
      patterns: [
        { pattern: "radix", confidence: "medium" },
        { pattern: "class-variance-authority", confidence: "medium" },
        { pattern: "@/components/ui/", confidence: "medium" },
        { pattern: "cn(", confidence: "low", notes: "clsx/cn utility" },
      ],
    },
    {
      name: "Radix UI",
      category: "ui",
      patterns: [
        { pattern: "data-radix-", confidence: "high" },
        { pattern: "@radix-ui/", confidence: "high" },
        { pattern: "data-state=", confidence: "medium" },
      ],
    },
    {
      name: "Element Plus",
      category: "ui",
      patterns: [
        { pattern: "el-", attribute: "class", confidence: "high" },
        { pattern: "el-button", confidence: "high" },
        { pattern: "el-input", confidence: "high" },
        { pattern: "element-plus", confidence: "high" },
      ],
    },
    {
      name: "Arco Design",
      category: "ui",
      patterns: [
        { pattern: "arco-", attribute: "class", confidence: "high" },
        { pattern: "@arco-design/", confidence: "high" },
      ],
    },
    {
      name: "Bootstrap",
      category: "ui",
      patterns: [
        { pattern: "btn-", attribute: "class", confidence: "medium" },
        { pattern: "navbar-", confidence: "medium" },
        { pattern: "container-", confidence: "medium" },
        { pattern: "col-", confidence: "medium" },
        { pattern: "row", confidence: "low" },
        { pattern: "bootstrap", confidence: "high" },
        { pattern: "/bootstrap.", confidence: "high" },
      ],
    },
    {
      name: "Bulma",
      category: "ui",
      patterns: [
        { pattern: "is-", attribute: "class", confidence: "medium" },
        { pattern: "has-", attribute: "class", confidence: "medium" },
        { pattern: "button is-", confidence: "high" },
        { pattern: "bulma", confidence: "high" },
      ],
    },
    {
      name: "Foundation",
      category: "ui",
      patterns: [
        { pattern: "foundation", confidence: "high" },
        { pattern: "grid-x", confidence: "medium" },
        { pattern: "cell", confidence: "low" },
      ],
    },
    {
      name: "Semantic UI",
      category: "ui",
      patterns: [
        { pattern: "ui ", attribute: "class", confidence: "medium" },
        { pattern: "semantic", confidence: "high" },
        { pattern: "fomantic-ui", confidence: "high" },
      ],
    },
    {
      name: "PrimeVue",
      category: "ui",
      patterns: [
        { pattern: "p-", attribute: "class", confidence: "medium" },
        { pattern: "primevue", confidence: "high" },
        { pattern: "primeflex", confidence: "medium" },
      ],
    },
    {
      name: "Vuetify",
      category: "ui",
      patterns: [
        { pattern: "v-", attribute: "class", confidence: "medium" },
        { pattern: "v-app", confidence: "high" },
        { pattern: "v-btn", confidence: "high" },
        { pattern: "vuetify", confidence: "high" },
      ],
    },
    {
      name: "Quasar",
      category: "ui",
      patterns: [
        { pattern: "q-", attribute: "class", confidence: "high" },
        { pattern: "quasar", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // STATE MANAGEMENT
  // ============================================
  state: [
    {
      name: "Redux",
      category: "state",
      patterns: [
        { pattern: "__REDUX_DEVTOOLS_EXTENSION__", confidence: "high" },
        { pattern: "createStore", confidence: "high" },
        { pattern: "configureStore", confidence: "high" },
        { pattern: "@reduxjs/", confidence: "high" },
        { pattern: "useDispatch", confidence: "high" },
        { pattern: "useSelector", confidence: "high" },
      ],
    },
    {
      name: "Zustand",
      category: "state",
      patterns: [
        { pattern: "zustand/vue", confidence: "high" },
        { pattern: "zustand", confidence: "high" },
        { pattern: "create(", confidence: "low" },
      ],
    },
    {
      name: "Pinia",
      category: "state",
      patterns: [
        { pattern: "pinia", confidence: "high" },
        { pattern: "defineStore", confidence: "high" },
        { pattern: "storePinia", confidence: "medium" },
      ],
    },
    {
      name: "MobX",
      category: "state",
      patterns: [
        { pattern: "mobx", confidence: "high" },
        { pattern: "@observable", confidence: "high" },
        { pattern: "@action", confidence: "high" },
        { pattern: "makeObservable", confidence: "high" },
      ],
    },
    {
      name: "Recoil",
      category: "state",
      patterns: [
        { pattern: "data-recoil-", confidence: "high" },
        { pattern: "recoil", confidence: "high" },
        { pattern: "useRecoilState", confidence: "high" },
        { pattern: "atom(", confidence: "high" },
        { pattern: "selector(", confidence: "high" },
      ],
    },
    {
      name: "Jotai",
      category: "state",
      patterns: [
        { pattern: "jotai", confidence: "high" },
        { pattern: "atom(", confidence: "low" },
        { pattern: "useAtom", confidence: "high" },
      ],
    },
    {
      name: "XState",
      category: "state",
      patterns: [
        { pattern: "xstate", confidence: "high" },
        { pattern: "createMachine", confidence: "high" },
        { pattern: "interpret(", confidence: "high" },
        { pattern: "useMachine", confidence: "high" },
      ],
    },
    {
      name: "Valtio",
      category: "state",
      patterns: [
        { pattern: "valtio", confidence: "high" },
        { pattern: "proxy(", confidence: "low" },
        { pattern: "useSnapshot", confidence: "high" },
      ],
    },
    {
      name: "Apollo Client",
      category: "state",
      patterns: [
        { pattern: "@apollo/client", confidence: "high" },
        { pattern: "useQuery", confidence: "medium" },
        { pattern: "useMutation", confidence: "medium" },
        { pattern: "ApolloClient", confidence: "high" },
      ],
    },
    {
      name: "TanStack Query",
      category: "state",
      patterns: [
        { pattern: "@tanstack/react-query", confidence: "high" },
        { pattern: "@tanstack/vue-query", confidence: "high" },
        { pattern: "useQuery", confidence: "medium" },
        { pattern: "QueryClient", confidence: "high" },
      ],
    },
    {
      name: "SWR",
      category: "state",
      patterns: [
        { pattern: "swr", confidence: "high" },
        { pattern: "useSWR", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // BUILD TOOLS
  // ============================================
  build: [
    {
      name: "Vite",
      category: "build",
      patterns: [
        { pattern: "/@vite/", confidence: "high" },
        { pattern: "type=\"module\"", confidence: "low" },
        { pattern: "import.meta.hot", confidence: "high" },
        { pattern: "@vitejs/", confidence: "high" },
      ],
    },
    {
      name: "Webpack",
      category: "build",
      patterns: [
        { pattern: "__webpack_require__", confidence: "high" },
        { pattern: "webpackChunkName", confidence: "high" },
        { pattern: "__webpack_public_path__", confidence: "high" },
        { pattern: /\/assets\/[\w\d]+\.[\w\d]+\.js/, confidence: "medium" },
      ],
    },
    {
      name: "Rollup",
      category: "build",
      patterns: [
        { pattern: "__ROLLUP__", confidence: "high" },
        { pattern: "rollup", confidence: "medium" },
      ],
    },
    {
      name: "esbuild",
      category: "build",
      patterns: [
        { pattern: "/* esbuild */", confidence: "high" },
        { pattern: "__esbuild", confidence: "medium" },
      ],
    },
    {
      name: "Parcel",
      category: "build",
      patterns: [
        { pattern: "parcel-bundler", confidence: "high" },
        { pattern: "parcel", confidence: "medium" },
      ],
    },
    {
      name: "Turbopack",
      category: "build",
      patterns: [
        { pattern: "__turbopack_", confidence: "high" },
        { pattern: "turbopack", confidence: "medium" },
      ],
    },
    {
      name: "Rspack",
      category: "build",
      patterns: [
        { pattern: "__rspack__", confidence: "high" },
        { pattern: "rspack", confidence: "medium" },
      ],
    },
    {
      name: "SWC",
      category: "build",
      patterns: [
        { pattern: "/* @swc */", confidence: "high" },
        { pattern: "@swc/", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // STYLING SOLUTIONS
  // ============================================
  styling: [
    {
      name: "CSS Modules",
      category: "styling",
      patterns: [
        { pattern: /--[a-zA-Z0-9]+--[a-zA-Z0-9]+/, confidence: "medium", notes: "Hash-based classes" },
        { pattern: ".module.", confidence: "high" },
        { pattern: "css-loader", confidence: "medium" },
      ],
    },
    {
      name: "Emotion",
      category: "styling",
      patterns: [
        { pattern: "css-{", confidence: "medium" },
        { pattern: "@emotion/react", confidence: "high" },
        { pattern: "@emotion/styled", confidence: "high" },
        { pattern: "css-prop", confidence: "medium" },
      ],
    },
    {
      name: "Styled Components",
      category: "styling",
      patterns: [
        { pattern: "styled.", confidence: "medium" },
        { pattern: "styled-components", confidence: "high" },
        { pattern: "css=", confidence: "low" },
      ],
    },
    {
      name: "Linaria",
      category: "styling",
      patterns: [
        { pattern: "/* linaria */", confidence: "high" },
        { pattern: "@linaria/", confidence: "high" },
      ],
    },
    {
      name: "Vanilla Extract",
      category: "styling",
      patterns: [
        { pattern: "@vanilla-extract/", confidence: "high" },
        { pattern: "style(", confidence: "low" },
      ],
    },
    {
      name: "SCSS/Sass",
      category: "styling",
      patterns: [
        { pattern: ".scss", confidence: "high" },
        { pattern: ".sass", confidence: "high" },
        { pattern: "sass-loader", confidence: "high" },
      ],
    },
    {
      name: "Less",
      category: "styling",
      patterns: [
        { pattern: ".less", confidence: "high" },
        { pattern: "less-loader", confidence: "high" },
      ],
    },
    {
      name: "Panda CSS",
      category: "styling",
      patterns: [
        { pattern: "panda-", confidence: "high" },
        { pattern: "@pandacss/", confidence: "high" },
        { pattern: "styled('panda')", confidence: "high" },
      ],
    },
    {
      name: "CSS-in-JS (Other)",
      category: "styling",
      patterns: [
        { pattern: "glamor", confidence: "high" },
        { pattern: "aphrodite", confidence: "high" },
        { pattern: "glamorous", confidence: "high" },
        { pattern: "styletron", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // BACKEND / API (Inferred from headers/patterns)
  // ============================================
  backend: [
    {
      name: "Express",
      category: "backend",
      patterns: [
        { pattern: "X-Powered-By: Express", confidence: "high" },
        { pattern: "express", confidence: "low" },
      ],
    },
    {
      name: "Fastify",
      category: "backend",
      patterns: [
        { pattern: "fastify", confidence: "medium" },
        { pattern: "X-Powered-By:", confidence: "low" },
      ],
    },
    {
      name: "NestJS",
      category: "backend",
      patterns: [
        { pattern: "nestjs", confidence: "medium" },
        { pattern: /"statusCode":\d{3}/, confidence: "low" },
      ],
    },
    {
      name: "Django",
      category: "backend",
      patterns: [
        { pattern: "csrftoken", confidence: "medium" },
        { pattern: "X-Frame-Options: SAMEORIGIN", confidence: "low" },
        { pattern: "django", confidence: "low" },
      ],
    },
    {
      name: "Rails",
      category: "backend",
      patterns: [
        { pattern: "X-Runtime:", confidence: "high" },
        { pattern: "csrf-token", confidence: "low" },
        { pattern: "rails", confidence: "low" },
      ],
    },
    {
      name: "Laravel",
      category: "backend",
      patterns: [
        { pattern: "XSRF-TOKEN", confidence: "medium" },
        { pattern: "laravel_session", confidence: "high" },
        { pattern: "laravel", confidence: "low" },
      ],
    },
    {
      name: "Spring Boot",
      category: "backend",
      patterns: [
        { pattern: "X-Application-Context:", confidence: "high" },
      ],
    },
    {
      name: "FastAPI",
      category: "backend",
      patterns: [
        { pattern: "/docs", confidence: "low", notes: "Check for Swagger UI" },
        { pattern: "/openapi.json", confidence: "low" },
      ],
    },
    {
      name: "Next.js API Routes",
      category: "backend",
      patterns: [
        { pattern: "/_next/api/", confidence: "high" },
      ],
    },
    {
      name: "Nuxt Server Routes",
      category: "backend",
      patterns: [
        { pattern: "/_nuxt/api/", confidence: "high" },
        { pattern: "/__nuxt_server_routes", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // HOSTING / INFRASTRUCTURE
  // ============================================
  hosting: [
    {
      name: "Vercel",
      category: "hosting",
      patterns: [
        { pattern: "x-vercel-", confidence: "high" },
        { pattern: "vercel", confidence: "medium" },
        { pattern: ".vercel.app", confidence: "high" },
      ],
    },
    {
      name: "Netlify",
      category: "hosting",
      patterns: [
        { pattern: "x-nf-", confidence: "high" },
        { pattern: "netlify", confidence: "medium" },
        { pattern: ".netlify.app", confidence: "high" },
      ],
    },
    {
      name: "Cloudflare",
      category: "hosting",
      patterns: [
        { pattern: "cf-ray", confidence: "high" },
        { pattern: "cf-", confidence: "high" },
        { pattern: "cloudflare", confidence: "medium" },
        { pattern: ".pages.dev", confidence: "high" },
        { pattern: ".workers.dev", confidence: "high" },
      ],
    },
    {
      name: "AWS",
      category: "hosting",
      patterns: [
        { pattern: "x-amz-", confidence: "high" },
        { pattern: "x-amzn-", confidence: "high" },
        { pattern: "x-amz-cf-id", confidence: "high", notes: "CloudFront" },
        { pattern: "s3", confidence: "medium" },
        { pattern: ".amazonaws.com", confidence: "medium" },
      ],
    },
    {
      name: "Azure",
      category: "hosting",
      patterns: [
        { pattern: "x-azure-", confidence: "high" },
        { pattern: ".azure", confidence: "medium" },
        { pattern: ".azurewebsites.net", confidence: "high" },
      ],
    },
    {
      name: "Google Cloud",
      category: "hosting",
      patterns: [
        { pattern: "x-google-", confidence: "high" },
        { pattern: ".appspot.com", confidence: "high" },
        { pattern: ".cloudfunctions.net", confidence: "high" },
      ],
    },
    {
      name: "Deno Deploy",
      category: "hosting",
      patterns: [
        { pattern: "x-deno-", confidence: "high" },
        { pattern: ".deno.dev", confidence: "high" },
      ],
    },
    {
      name: "Railway",
      category: "hosting",
      patterns: [
        { pattern: "x-railway-", confidence: "high" },
        { pattern: ".railway.app", confidence: "high" },
      ],
    },
    {
      name: "Fly.io",
      category: "hosting",
      patterns: [
        { pattern: "x-fly-", confidence: "high" },
        { pattern: ".fly.dev", confidence: "high" },
      ],
    },
    {
      name: "Render",
      category: "hosting",
      patterns: [
        { pattern: "x-render-", confidence: "high" },
        { pattern: ".onrender.com", confidence: "high" },
      ],
    },
    {
      name: "Heroku",
      category: "hosting",
      patterns: [
        { pattern: "x-heroku-", confidence: "high" },
        { pattern: ".herokuapp.com", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // ANALYTICS
  // ============================================
  analytics: [
    {
      name: "Google Analytics",
      category: "analytics",
      patterns: [
        { pattern: "GA_MEASUREMENT_ID", confidence: "high" },
        { pattern: "gtag(", confidence: "high" },
        { pattern: "dataLayer", confidence: "high" },
        { pattern: "googletagmanager.com", confidence: "high" },
        { pattern: "google-analytics.com", confidence: "high" },
      ],
    },
    {
      name: "Plausible",
      category: "analytics",
      patterns: [
        { pattern: "plausible.io/js/script.js", confidence: "high" },
        { pattern: "plausible(", confidence: "high" },
      ],
    },
    {
      name: "Fathom",
      category: "analytics",
      patterns: [
        { pattern: "fathom.js", confidence: "high" },
        { pattern: "fathom.trackEvent", confidence: "high" },
      ],
    },
    {
      name: "PostHog",
      category: "analytics",
      patterns: [
        { pattern: "posthog", confidence: "high" },
        { pattern: "posthog-js", confidence: "high" },
      ],
    },
    {
      name: "Segment",
      category: "analytics",
      patterns: [
        { pattern: "analytics.", confidence: "low" },
        { pattern: "segment.com", confidence: "high" },
        { pattern: "analytics-js", confidence: "high" },
      ],
    },
    {
      name: "Hotjar",
      category: "analytics",
      patterns: [
        { pattern: "hj", confidence: "medium" },
        { pattern: "static.hotjar.com", confidence: "high" },
      ],
    },
    {
      name: "FullStory",
      category: "analytics",
      patterns: [
        { pattern: "FS", confidence: "low" },
        { pattern: "__fs", confidence: "high" },
        { pattern: "fullstory.com", confidence: "high" },
      ],
    },
    {
      name: "Mixpanel",
      category: "analytics",
      patterns: [
        { pattern: "mixpanel", confidence: "high" },
        { pattern: "mixpanel-js", confidence: "high" },
      ],
    },
    {
      name: "Amplitude",
      category: "analytics",
      patterns: [
        { pattern: "amplitude", confidence: "high" },
        { pattern: "analytics.amplitude.com", confidence: "high" },
      ],
    },
    {
      name: "Heap",
      category: "analytics",
      patterns: [
        { pattern: "heap", confidence: "medium" },
        { pattern: "heapanalytics.com", confidence: "high" },
      ],
    },
    {
      name: "Umami",
      category: "analytics",
      patterns: [
        { pattern: "umami", confidence: "high" },
        { pattern: "umami.is", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // MONITORING / ERROR TRACKING
  // ============================================
  monitoring: [
    {
      name: "Sentry",
      category: "monitoring",
      patterns: [
        { pattern: "Sentry", confidence: "high" },
        { pattern: "__SENTRY__", confidence: "high" },
        { pattern: "browser.sentry-cdn.com", confidence: "high" },
        { pattern: "sentry.io", confidence: "high" },
      ],
    },
    {
      name: "LogRocket",
      category: "monitoring",
      patterns: [
        { pattern: "LogRocket", confidence: "high" },
        { pattern: "logrocket.com", confidence: "high" },
      ],
    },
    {
      name: "Bugsnag",
      category: "monitoring",
      patterns: [
        { pattern: "Bugsnag", confidence: "high" },
        { pattern: "bugsnag.com", confidence: "high" },
      ],
    },
    {
      name: "Datadog",
      category: "monitoring",
      patterns: [
        { pattern: "DD_LOGS", confidence: "high" },
        { pattern: "DD_RUM", confidence: "high" },
        { pattern: "dd-", confidence: "low" },
        { pattern: "datadoghq.com", confidence: "high" },
      ],
    },
    {
      name: "Rollbar",
      category: "monitoring",
      patterns: [
        { pattern: "Rollbar", confidence: "high" },
        { pattern: "rollbar.com", confidence: "high" },
      ],
    },
    {
      name: "Airbrake",
      category: "monitoring",
      patterns: [
        { pattern: "airbrake", confidence: "high" },
        { pattern: "airbrake.io", confidence: "high" },
      ],
    },
  ],

  // ============================================
  // AUTHENTICATION
  // ============================================
  auth: [
    {
      name: "Auth0",
      category: "auth",
      patterns: [
        { pattern: "auth0", confidence: "high" },
        { pattern: "cdn.auth0.com", confidence: "high" },
        { pattern: "wp-", confidence: "low", notes: "Webtask, check context" },
      ],
    },
    {
      name: "Firebase Auth",
      category: "auth",
      patterns: [
        { pattern: "firebase.auth()", confidence: "high" },
        { pattern: "firebase", confidence: "medium" },
      ],
    },
    {
      name: "Clerk",
      category: "auth",
      patterns: [
        { pattern: "Clerk", confidence: "high" },
        { pattern: "clerk.com", confidence: "high" },
        { pattern: "clerk-", attribute: "class", confidence: "medium" },
      ],
    },
    {
      name: "NextAuth",
      category: "auth",
      patterns: [
        { pattern: "next-auth", confidence: "high" },
        { pattern: "signIn(", confidence: "low" },
        { pattern: "getServerSession", confidence: "medium" },
      ],
    },
    {
      name: "Supabase Auth",
      category: "auth",
      patterns: [
        { pattern: "supabase.auth", confidence: "high" },
        { pattern: "supabase", confidence: "medium" },
      ],
    },
    {
      name: "Lucia Auth",
      category: "auth",
      patterns: [
        { pattern: "lucia-auth", confidence: "high" },
        { pattern: "lucia", confidence: "medium" },
      ],
    },
  ],

  // ============================================
  // PAYMENT
  // ============================================
  payment: [
    {
      name: "Stripe",
      category: "payment",
      patterns: [
        { pattern: "Stripe(", confidence: "high" },
        { pattern: "stripe.", confidence: "high" },
        { pattern: "stripe.com", confidence: "high" },
        { pattern: "js.stripe.com", confidence: "high" },
      ],
    },
    {
      name: "PayPal",
      category: "payment",
      patterns: [
        { pattern: "paypal", confidence: "high" },
        { pattern: "paypalobjects.com", confidence: "high" },
      ],
    },
    {
      name: "Shopify",
      category: "payment",
      patterns: [
        { pattern: "Shopify", confidence: "high" },
        { pattern: "shopify.com", confidence: "high" },
        { pattern: "cdn.shopify.com", confidence: "high" },
      ],
    },
  ],
}

/**
 * Find technologies matching a given content string
 */
export function findFingerprints(content: string): Map<string, { tech: TechnologyFingerprint; matches: string[] }> {
  const results = new Map<string, { tech: TechnologyFingerprint; matches: string[] }>()
  const lowerContent = content.toLowerCase()

  for (const [category, techs] of Object.entries(FINGERPRINTS)) {
    for (const tech of techs) {
      const matches: string[] = []

      for (const pattern of tech.patterns) {
        const patterns = Array.isArray(pattern.pattern) ? pattern.pattern : [pattern.pattern]

        for (const p of patterns) {
          let found = false

          if (p instanceof RegExp) {
            if (p.test(content) || p.test(lowerContent)) {
              found = true
            }
          } else if (typeof p === "string") {
            if (content.includes(p) || lowerContent.includes(p.toLowerCase())) {
              found = true
            }
          }

          if (found) {
            matches.push(`${String(p)} (${pattern.confidence})`)
          }
        }
      }

      if (matches.length > 0) {
        results.set(tech.name, { tech, matches })
      }
    }
  }

  return results
}

/**
 * Get all technologies for a category
 */
export function getFingerprintsByCategory(category: string): TechnologyFingerprint[] {
  return FINGERPRINTS[category] || []
}

/**
 * Get all technology categories
 */
export function getCategories(): string[] {
  return Object.keys(FINGERPRINTS)
}
