// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// https://astro.build/config
export default defineConfig({
	site: 'https://articwake.pages.dev',
	integrations: [
		starlight({
			title: 'articwake',
			description: 'Remote Wake-on-LAN and LUKS unlock for homelab servers',
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/a-maccormack/articwake' },
			],
			editLink: {
				baseUrl: 'https://github.com/a-maccormack/articwake/edit/main/docs/',
			},
			customCss: ['./src/styles/custom.css'],
			sidebar: [
				{
					label: 'Getting Started',
					items: [
						{ label: 'Introduction', slug: 'index' },
						{ label: 'Quick Start', slug: 'getting-started/quick-start' },
						{ label: 'Installation', slug: 'getting-started/installation' },
					],
				},
				{
					label: 'User Guide',
					items: [
						{ label: 'SD Card Setup', slug: 'guides/sd-card-setup' },
						{ label: 'Configuration', slug: 'guides/configuration' },
						{ label: 'WiFi Setup', slug: 'guides/wifi-setup' },
						{ label: 'Tailscale Integration', slug: 'guides/tailscale' },
						{ label: 'Web UI Usage', slug: 'guides/web-ui' },
					],
				},
				{
					label: 'API Reference',
					items: [
						{ label: 'Overview', slug: 'api/overview' },
						{ label: 'Authentication', slug: 'api/auth' },
						{ label: 'Status', slug: 'api/status' },
						{ label: 'Wake-on-LAN', slug: 'api/wol' },
						{ label: 'LUKS Unlock', slug: 'api/unlock' },
					],
				},
				{
					label: 'Homelab Setup',
					items: [
						{ label: 'Wake-on-LAN', slug: 'homelab/wol' },
						{ label: 'LUKS with Dropbear', slug: 'homelab/luks-dropbear' },
						{ label: 'SSH Key Setup', slug: 'homelab/ssh-keys' },
					],
				},
				{
					label: 'Developer Guide',
					items: [
						{ label: 'Architecture', slug: 'development/architecture' },
						{ label: 'Building from Source', slug: 'development/building' },
						{ label: 'Testing', slug: 'development/testing' },
						{ label: 'Contributing', slug: 'development/contributing' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'Environment Variables', slug: 'reference/environment' },
						{ label: 'CLI Commands', slug: 'reference/cli' },
						{ label: 'Security', slug: 'reference/security' },
						{ label: 'Troubleshooting', slug: 'reference/troubleshooting' },
					],
				},
			],
		}),
	],
});
