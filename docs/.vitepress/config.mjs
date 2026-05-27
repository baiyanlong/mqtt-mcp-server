import { defineConfig } from 'vitepress'

export default defineConfig({
  lang: 'zh-CN',
  title: 'MQTT MCP Server',
  description: '云端 AI 操控物理设备 — Rust 实现，5 分钟部署',

  themeConfig: {
    nav: [
      { text: '首页', link: '/' },
      { text: '指南', link: '/guide/getting-started' },
      { text: '部署', link: '/deploy/raspberry-pi' },
      { text: 'API', link: '/api/tools' },
      { text: 'FAQ', link: '/faq' },
      { text: 'GitHub', link: 'https://github.com/baiyanlong/mqtt-mcp-server' },
    ],

    sidebar: {
      '/guide/': [
        { text: '快速开始', link: '/guide/getting-started' },
        { text: '什么是 MCP', link: '/guide/what-is-mcp' },
        { text: '架构', link: '/guide/architecture' },
        { text: 'AI 配置', link: '/guide/ai-config' },
        { text: '规则引擎', link: '/guide/rule-engine' },
      ],
      '/deploy/': [
        { text: '树莓派 / ARM64', link: '/deploy/raspberry-pi' },
        { text: 'Docker', link: '/deploy/docker' },
        { text: '从源码编译', link: '/deploy/build-from-source' },
      ],
      '/api/': [
        { text: 'MCP 工具参考', link: '/api/tools' },
        { text: 'SSE 端点', link: '/api/sse' },
        { text: 'CLI 参数', link: '/api/cli' },
        { text: 'Cloud API (Pro)', link: '/api/cloud' },
      ],
    },

    footer: {
      message: 'MIT Licensed | by byl',
      copyright: 'Copyright © 2026',
    },

    search: {
      provider: 'local',
    },
  },
})
