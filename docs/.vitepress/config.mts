import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  base: '/AxumKit/',
  title: "AxumKit",
  description: "Production-ready Rust web backend template.",
  ignoreDeadLinks: [
    /^http:\/\/localhost/,
  ],
  themeConfig: {
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Features', link: '/features/authentication' },
      { text: 'Reference', link: '/reference/api-endpoints' },
      { text: 'Deploy', link: '/deploy/docker' },
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Guide',
          items: [
            { text: 'Getting Started', link: '/guide/getting-started' },
            { text: 'Study Guide', link: '/guide/study-guide' },
            { text: 'Architecture', link: '/guide/architecture' },
            { text: 'Configuration', link: '/guide/configuration' },
          ],
        },
      ],
      '/features/': [
        {
          text: 'Features',
          items: [
            { text: 'Authentication', link: '/features/authentication' },
            { text: 'OAuth2', link: '/features/oauth' },
            { text: 'TOTP 2FA', link: '/features/totp' },
            { text: 'Posts', link: '/features/posts' },
            { text: 'Search', link: '/features/search' },
            { text: 'Background Worker', link: '/features/worker' },
            { text: 'Rate Limiting', link: '/features/rate-limiting' },
          ],
        },
      ],
      '/reference/': [
        {
          text: 'Reference',
          items: [
            { text: 'API Endpoints', link: '/reference/api-endpoints' },
            { text: 'Error Codes', link: '/reference/error-codes' },
            { text: 'Environment Variables', link: '/reference/environment' },
          ],
        },
      ],
      '/deploy/': [
        {
          text: 'Deployment',
          items: [
            { text: 'Docker', link: '/deploy/docker' },
          ],
        },
      ],
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/levish0/AxumKit' },
    ],

    search: {
      provider: 'local',
    },

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright 2025-present AxumKit Contributors',
    },
  },
})
