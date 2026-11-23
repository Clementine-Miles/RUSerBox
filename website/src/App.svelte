<script>
  import { Button } from '@serverbox/webui'
  import {
    ExternalLink,
  } from '@lucide/svelte'
  import { spring } from 'svelte/motion'
  import { onMount } from 'svelte'
  import LL, { setLocale } from './i18n/i18n-svelte'
  import { loadLocale } from './i18n/i18n-util.sync'
  import {
    docsHref,
    getInitialLocale,
    locales,
    persistUserLocale,
    syncLocaleToUrl,
  } from './lib/i18n'
  // One per screenshot, in the order of the README's table. `tag` is what the
  // screen talks to on the server, so it stays untranslated.
  const features = [
    { key: 'list', shot: 'home', tag: 'ssh · monitor' },
    { key: 'charts', shot: 'server-details', tag: '/proc · /sys' },
    { key: 'terminal', shot: 'terminal', tag: 'ssh' },
    { key: 'files', shot: 'files', tag: 'sftp' },
    { key: 'containers', shot: 'container', tag: 'docker · podman' },
    { key: 'processes', shot: 'process', tag: 'ps' },
    { key: 'services', shot: 'services', tag: 'systemctl' },
    { key: 'users', shot: 'users', tag: '/etc/passwd' },
    { key: 'agent', shot: 'agent', tag: 'llm · mcp' },
    { key: 'bench', shot: 'bench', tag: 'yabs' },
    { key: 'virt', shot: 'virt', tag: 'libvirt · pve' },
  ]

  // Which feature the phone shows: the one scrolled to the middle of the
  // viewport, or the one last pressed.
  let activeFeature = $state(0)
  let featureItems = $state([])

  // The item whose middle is nearest the viewport's middle, read on every
  // scroll frame: a band-crossing observer missed items a fast wheel jumped.
  $effect(() => {
    const items = featureItems.filter(Boolean)
    if (!items.length) return
    let frame = 0
    const update = () => {
      frame = 0
      // The phone column is hidden below 860px; nothing to follow then.
      if (!window.matchMedia('(min-width: 861px)').matches) return
      const middle = window.innerHeight / 2
      let best = activeFeature
      let bestDistance = Infinity
      items.forEach((item, index) => {
        const rect = item.getBoundingClientRect()
        const distance = Math.abs(rect.top + rect.height / 2 - middle)
        if (distance < bestDistance) {
          bestDistance = distance
          best = index
        }
      })
      activeFeature = best
    }
    const onScroll = () => {
      if (!frame) frame = requestAnimationFrame(update)
    }
    window.addEventListener('scroll', onScroll, { passive: true })
    update()
    return () => {
      window.removeEventListener('scroll', onScroll)
      if (frame) cancelAnimationFrame(frame)
    }
  })

  // The CDN stores original PNGs and 1440px JPEGs for the web under matching
  // names, so refreshing an image does not require a source-code change.
  const shotBase = 'https://cdn.lollipopkit.com/serverbox/screenshot'

  // The order matches the stack transforms below. Each key also selects its
  // localized alt text, so update all locales when changing a key's image.
  const screenshots = [
    { src: `${shotBase}/iphone/home.jpg`, key: 'one', x: -24, y: 8, hoverSlot: -1.5, rotate: -8, hoverRotate: -1.8, motion: 18 },
    { src: `${shotBase}/iphone/server-details.jpg`, key: 'two', x: -8, y: -4, hoverSlot: -0.5, rotate: -2.5, hoverRotate: -0.6, motion: 12 },
    { src: `${shotBase}/iphone/terminal.jpg`, key: 'three', x: 9, y: 4, hoverSlot: 0.5, rotate: 3, hoverRotate: 0.6, motion: 14 },
    { src: `${shotBase}/iphone/files.jpg`, key: 'four', x: 24, y: -2, hoverSlot: 1.5, rotate: 8.5, hoverRotate: 1.8, motion: 20 },
  ]

  const repository = 'https://github.com/Clementine-Miles/ru_serbox'
  const releases = `${repository}/releases`
  const moreServiceList = [
    { key: 'llmapi', href: 'https://cpa.lolli.tech' },
    { key: 'uptimer', href: 'https://up.lolli.tech' },
  ]
  const otherAppList = [
    { key: 'llmbox', href: 'https://github.com/lollipopkit/llmbox' },
    { key: 'mfuse', href: 'https://mfuse.lollipopkit.com' },
  ]

  const stackMotion = spring(
    { x: 0, y: 0 },
    {
      stiffness: 0.12,
      damping: 0.38,
    },
  )

  function getLocaleBeforeRender() {
    if (typeof window === 'undefined') return undefined

    return getInitialLocale()
  }

  const initialLocale = getLocaleBeforeRender()

  if (initialLocale) {
    loadLocale(initialLocale)
    setLocale(initialLocale)
  }

  let locale = $state(initialLocale)

  function applyLocale(nextLocale) {
    locale = nextLocale
    loadLocale(nextLocale)
    setLocale(nextLocale)
  }

  onMount(() => {
    const nextLocale = locale || getInitialLocale()
    applyLocale(nextLocale)
    syncLocaleToUrl(nextLocale)
  })

  $effect(() => {
    if (!locale) return

    document.documentElement.lang = $LL.meta.lang()
    document.title = $LL.meta.title()
    document
      .querySelector('meta[name="description"]')
      ?.setAttribute('content', $LL.meta.description())
  })

  function handleLocaleChange(event) {
    const nextLocale = event.currentTarget.value
    applyLocale(nextLocale)
    persistUserLocale(nextLocale)
    syncLocaleToUrl(nextLocale, true)
  }

  function handleStackMove(event) {
    const rect = event.currentTarget.getBoundingClientRect()
    const x = (event.clientX - rect.left) / rect.width - 0.5
    const y = (event.clientY - rect.top) / rect.height - 0.5

    stackMotion.set({ x, y })
  }

  function resetStack() {
    stackMotion.set({ x: 0, y: 0 })
  }

  // Smooth unless the reader asked for reduced motion; the CSS rule for it
  // covers transitions, not scrolling started from here.
  function scrollBehavior() {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches ? 'auto' : 'smooth'
  }

  function scrollToSection(event, id) {
    event.preventDefault()
    document.getElementById(id)?.scrollIntoView({
      behavior: scrollBehavior(),
      block: 'start',
    })
  }

</script>

{#if locale}
  <main class="site">
    <header class="site-nav" id="top">
      <a class="brand" href="#top" onclick={(event) => scrollToSection(event, 'top')}>
        <img src="/app_icon.png" alt="" width="26" height="26" />
        ServerBox
      </a>
      <nav>
        <a href="#features" onclick={(event) => scrollToSection(event, 'features')}>{$LL.nav.features()}</a>
        <a href="#download" onclick={(event) => scrollToSection(event, 'download')}>{$LL.nav.download()}</a>
        <a href="#other-apps" onclick={(event) => scrollToSection(event, 'other-apps')}>{$LL.nav.otherApps()}</a>
        <a href={docsHref(locale)}>{$LL.nav.docs()}</a>
      </nav>
      <div class="nav-actions">
        <label class="language-switcher">
          <span class="sr-only">{$LL.nav.languageLabel()}</span>
          <select
            id="locale"
            name="locale"
            aria-label={$LL.nav.languageLabel()}
            value={locale}
            onchange={handleLocaleChange}
          >
            {#each locales as item}
              <option value={item.code}>{item.label}</option>
            {/each}
          </select>
        </label>
        <a class="nav-cta" href="https://api.lolli.tech">{$LL.nav.signIn()}</a>
      </div>
    </header>

    <section class="hero" id="hero">
      <div class="hero-copy">
        <h1>{$LL.hero.titlePrefix()}<br /><span>{$LL.hero.titleSuffix()}</span></h1>
        <p class="hero-subtitle">
          {$LL.hero.subtitle()}
        </p>
        <div class="hero-actions">
          <Button href="#download" onclick={(event) => scrollToSection(event, 'download')}>{$LL.hero.primaryAction()}</Button>
          <Button variant="secondary" href="#features" onclick={(event) => scrollToSection(event, 'features')}>{$LL.hero.secondaryAction()}</Button>
        </div>
      </div>

      <div
        class="screenshot-stack"
        aria-label={$LL.screenshots.label()}
        onmousemove={handleStackMove}
        onmouseleave={resetStack}
        role="img"
      >
        {#each screenshots as shot, index}
          <img
            class="screenshot-card"
            src={shot.src}
            alt={$LL.screenshots[shot.key]()}
            loading={index === 0 ? 'eager' : 'lazy'}
            referrerpolicy="no-referrer"
            style={`--base-x:${shot.x}%; --base-y:${shot.y}%; --hover-slot:${shot.hoverSlot}; --base-r:${shot.rotate}deg; --hover-r:${shot.hoverRotate}deg; --move-x:${$stackMotion.x * shot.motion}px; --move-y:${$stackMotion.y * shot.motion}px; --tilt-x:${-$stackMotion.y * 6}deg; --tilt-y:${$stackMotion.x * 8}deg; --z:${screenshots.length - index};`}
          />
        {/each}
      </div>
    </section>

    <section class="page-section" id="features">
      <div class="section-head">
        <h2>{$LL.features.title()}</h2>
        <p>
          {$LL.features.subtitle()}
        </p>
      </div>

      <div class="tour">
        <ol class="tour-list">
          {#each features as feature, index}
            <li bind:this={featureItems[index]}>
              <button
                type="button"
                class="tour-item"
                class:active={index === activeFeature}
                aria-pressed={index === activeFeature}
                onclick={() => {
                  activeFeature = index
                  featureItems[index]?.scrollIntoView({ behavior: scrollBehavior(), block: 'center' })
                }}
              >
                <span class="tour-tag">{String(index + 1).padStart(2, '0')} · {feature.tag}</span>
                <span class="tour-title">{$LL.features[feature.key].title()}</span>
                <span class="tour-desc">{$LL.features[feature.key].description()}</span>
              </button>
              <!-- Phones get the picture beside its text; wider screens, the
                   one device frame. -->
              <img
                class="tour-shot-inline"
                src={`${shotBase}/iphone/${feature.shot}.jpg`}
                alt=""
                loading="lazy"
                decoding="async"
                referrerpolicy="no-referrer"
              />
            </li>
          {/each}
        </ol>
        <div class="tour-device">
          <div class="device-frame">
            {#each features as feature, index}
              <img
                class:shown={index === activeFeature}
                src={`${shotBase}/iphone/${feature.shot}.jpg`}
                alt={index === activeFeature ? $LL.features[feature.key].title() : ''}
                loading="lazy"
                decoding="async"
                referrerpolicy="no-referrer"
              />
            {/each}
          </div>
        </div>
      </div>
    </section>

    <section class="download-section" id="download">
      <div class="section-head">
        <h2>{$LL.download.title()}</h2>
        <p>
          {$LL.download.subtitle()}
        </p>
      </div>

      <div class="download-list">
        <article class="download-platform">
          <div class="download-platform-copy"><h3>Android</h3></div>
          <div class="download-actions">
            <a class="download-icon-btn" href={releases}>
              <span>{$LL.cta.githubAction()}</span>
              <small>.apk arm64 / arm / amd64</small>
              <ExternalLink size={14} strokeWidth={1.8} aria-hidden="true" />
            </a>
          </div>
        </article>
      </div>

      <p class="download-note">{$LL.download.note()} {$LL.cta.title()}</p>
    </section>

    <section class="page-section other-apps-section" id="other-apps">
      <div class="section-head">
        <h2>{$LL.otherApps.title()}</h2>
        <p>{$LL.otherApps.subtitle()}</p>
      </div>

      <div class="more-group">
        <h3 class="more-group-title">{$LL.otherApps.servicesHeading()}</h3>
        <div class="other-apps-grid">
          {#each moreServiceList as service}
            <article class="feature-card other-app-card">
              <div class="other-app-head">
                <h4>
                  <a class="other-app-title-link" href={service.href}>
                    <span>{$LL.otherApps[service.key].title()}</span>
                    <ExternalLink size={15} strokeWidth={1.8} aria-hidden="true" />
                  </a>
                </h4>
              </div>
              <p>{$LL.otherApps[service.key].description()}</p>
            </article>
          {/each}
        </div>
      </div>

      <div class="more-group">
        <h3 class="more-group-title">{$LL.otherApps.appsHeading()}</h3>
        <div class="other-apps-grid">
          {#each otherAppList as app}
            <article class="feature-card other-app-card">
              <div class="other-app-head">
                <h4>{$LL.otherApps[app.key].title()}</h4>
                <span class="protocol-badge">{$LL.otherApps[app.key].tag()}</span>
              </div>
              <p>{$LL.otherApps[app.key].description()}</p>
              <div class="download-links">
                <a class="download-icon-btn" href={app.href}>
                  <span>{$LL.otherApps[app.key].action()}</span>
                  <ExternalLink size={14} strokeWidth={1.8} aria-hidden="true" />
                </a>
              </div>
            </article>
          {/each}
        </div>
      </div>
    </section>

    <footer class="site-footer">
      <span>© 2026 lollipopkit</span>
      <div class="footer-links">
        <a href="#features" onclick={(event) => scrollToSection(event, 'features')}>{$LL.footer.features()}</a>
        <a href="#other-apps" onclick={(event) => scrollToSection(event, 'other-apps')}>{$LL.nav.otherApps()}</a>
        <a href={docsHref(locale)}>{$LL.nav.docs()}</a>
        <a href={repository}>GitHub</a>
        <a href={releases}>{$LL.footer.releases()}</a>
      </div>
    </footer>
  </main>
{/if}
