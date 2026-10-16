# web/ Layout

Elixir Phoenix/Hologram landing site for **dirconfig.therrobotlives.com** (`:direnv_config_web`). Server-rendered, no DB/auth; containerised via `web/Dockerfile`, deployed by `helm/dirconfig-site/`.

```
web/
├── app/                        # Hologram pages/components (compiled to priv/static/hologram JS)
│   ├── main_layout.ex          #   Page layout module
│   └── pages/
│       ├── home_page.ex        #   Home page logic
│       └── home_page.holo      #   Home page template
├── lib/direnv_config_web/      # Phoenix server modules
│   ├── application.ex          #   OTP application
│   ├── endpoint.ex             #   Plug endpoint
│   ├── router.ex               #   Routes (single landing route + health)
│   ├── controllers/health.ex   #   Health check endpoint
│   ├── plugs/session.ex        #   Session plug
│   └── error_html.ex           #   Error page rendering
├── config/                     # Mix config (config.exs, runtime.exs, test.exs)
├── priv/static/                # Compiled Hologram JS bundles + CSS/favicon (build output, tracked)
├── test/                       # Web test suite
├── mix.exs / mix.lock          # Package manifest (Phoenix ~> 1.8 + Hologram compiler)
├── Dockerfile                  # Multi-stage build (final stage MUST be named `production`)
└── COPY.md                     # Landing-page copy deck — source of truth for page text
```
