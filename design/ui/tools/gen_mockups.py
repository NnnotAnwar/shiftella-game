#!/usr/bin/env python3
"""SVG mockup generator for the Shiftella UI.

Run:  python3 -I design/ui/tools/gen_mockups.py
Writes *.svg into design/ui/. All coordinates are logical points (pt)
on the 1280×720 reference screen. The values match screens.md.

These are the pre-implementation (v2) layouts. The palette was switched to the
shipped Shiftella indigo/violet tokens (src/ui/theme.rs); the world art still
shows the old strictly top-down look, not the shipped 3/4 "violet dollhouse".
"""
import math
import os
import random
from xml.sax.saxutils import escape

OUT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
W, H = 1280, 720

# ---------------------------------------------------------------- tokens
C = {
    'bg0': '#0B0A18',        # deepest background / void
    'bg1': '#12112A',        # menu screen background
    'panel': '#17162E',      # panel (rgba 90 %)
    'panel_top': '#1E1C3A',  # top of the panel gradient
    'raised': '#24223F',     # field, button
    'hover': '#2E2B52',      # hover
    'line': '#34315C',       # dividers
    'text': '#EEEDF7',
    'text2': '#A9A7C9',
    'muted': '#6F6D97',
    'accent': '#6366F1',     # Shiftella indigo — the only accent
    'accent_hi': '#A5B4FC',
    'accent_lo': '#4F46E5',
    'cyan': '#818CF8',       # info / time (indigo glow)
    'violet': '#7C3AED',     # start of the night on the progress bar
    'amber': '#FFC46B',      # money, warm light
    'honey': '#F99C4E',
    'good': '#34D399',
    'warn': '#FFCC4D',
    'bad': '#F87171',
    'skin1': '#F2CBA8', 'skin2': '#D9A47C', 'skin3': '#A8714E',
}
FONT_UI = "Inter, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif"
FONT_HEAD = "Inter, 'Segoe UI', 'Helvetica Neue', Arial, sans-serif"


def tw(s, size=13, bold=False):
    """Rough width estimate of an Inter string (~0.57 em per character)."""
    return len(s) * size * (0.6 if bold else 0.565)


def col(c):
    return C.get(c, c)


# ---------------------------------------------------------------- icons
# 24×24, stroke 2, round caps/joins, fill none. One style for the whole game.
ICONS = {
    'clock': '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/>',
    'pause': '<path d="M9 6v12M15 6v12"/>',
    'play': '<path d="M8 5.5l10 6.5-10 6.5z"/>',
    'ff2': '<path d="M4 6.5l7.5 5.5L4 17.5zM12.5 6.5L20 12l-7.5 5.5z"/>',
    'ff3': '<path d="M2.5 7.5l5 4.5-5 4.5zM9.5 7.5l5 4.5-5 4.5zM16.5 7.5l5 4.5-5 4.5z"/>',
    'coin': '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="4.5"/>',
    'star': '<path d="M12 3.5l2.6 5.4 5.9.8-4.3 4.1 1 5.8L12 16.8l-5.2 2.8 1-5.8-4.3-4.1 5.9-.8z"/>',
    'staff': '<circle cx="9" cy="8" r="3.5"/><path d="M3 20c0-3.5 2.7-6 6-6s6 2.5 6 6"/><circle cx="17" cy="9" r="2.5"/><path d="M16.5 14c2.6.2 4.5 2.2 4.5 5"/>',
    'guest': '<circle cx="12" cy="8" r="4"/><path d="M4.5 21c0-4.2 3.4-7 7.5-7s7.5 2.8 7.5 7"/>',
    'prio': '<rect x="3.5" y="3.5" width="7" height="7" rx="1.5"/><rect x="13.5" y="3.5" width="7" height="7" rx="1.5"/><rect x="3.5" y="13.5" width="7" height="7" rx="1.5"/><path d="M14 17h6M17 14v6"/>',
    'calendar': '<rect x="3.5" y="5" width="17" height="15.5" rx="2.5"/><path d="M3.5 10h17M8 3v4M16 3v4M7.5 14h4M12.5 17h4"/>',
    'hammer': '<path d="M13.5 4.5l6 6-2.5 2.5-6-6z"/><path d="M12 9L4.5 16.5a1.8 1.8 0 0 0 2.5 2.5L14.5 11.5"/>',
    'log': '<path d="M9 6.5h11M9 12h11M9 17.5h11M4.5 6.5h.01M4.5 12h.01M4.5 17.5h.01"/>',
    'menu': '<path d="M4 7h16M4 12h16M4 17h16"/>',
    'bolt': '<path d="M13 3L5 13.5h6L10 21l8-10.5h-6z"/>',
    'smile': '<circle cx="12" cy="12" r="9"/><path d="M8.5 14.5c1 1.2 2.2 1.8 3.5 1.8s2.5-.6 3.5-1.8M9 9.5h.01M15 9.5h.01"/>',
    'bowl': '<path d="M3.5 12h17a8.5 8.5 0 0 1-17 0z"/><path d="M9 3.5c-1 1.4 1 2.4 0 4M15 3.5c-1 1.4 1 2.4 0 4"/>',
    'pan': '<circle cx="10" cy="13" r="7"/><circle cx="10" cy="13" r="2.5"/><path d="M17 13h4.5"/>',
    'cup': '<path d="M4 9h12v4.5a5.5 5.5 0 0 1-5.5 5.5h-1A5.5 5.5 0 0 1 4 13.5z"/><path d="M16 10.5h1.5a2.5 2.5 0 0 1 0 5H16M8 3c-.8 1.2.8 2 0 3.2M12 3c-.8 1.2.8 2 0 3.2"/>',
    'register': '<rect x="3" y="11" width="18" height="9.5" rx="2"/><rect x="6.5" y="3.5" width="9" height="5" rx="1"/><path d="M7 15h.01M11 15h.01M15 15h.01M7 17.5h10"/>',
    'broom': '<path d="M16 3l-5 8"/><path d="M7.5 11h7l3 9.5h-13z"/><path d="M9.5 15l-1 5.5M12.5 15v5.5"/>',
    'bulb_off': '<path d="M9.5 18h5M10.5 21h3"/><path d="M12 3a6 6 0 0 0-3.6 10.8c.7.5 1.1 1.3 1.1 2.2h5c0-.9.4-1.7 1.1-2.2A6 6 0 0 0 12 3z"/><path d="M4 4l16 16"/>',
    'train': '<rect x="5" y="3" width="14" height="14" rx="3.5"/><path d="M5 10.5h14M9 21l1.5-4M15 21l-1.5-4M9 13.8h.01M15 13.8h.01"/>',
    'rain': '<path d="M7 14.5a4 4 0 0 1 .4-8 5.5 5.5 0 0 1 10.6 1.5 3.3 3.3 0 0 1-.5 6.5z"/><path d="M8.5 18l-1 2.5M12.5 18l-1 2.5M16.5 18l-1 2.5"/>',
    'warn': '<path d="M12 4l9 16H3z"/><path d="M12 10v4M12 17h.01"/>',
    'info': '<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h.01"/>',
    'close': '<path d="M6.5 6.5l11 11M17.5 6.5l-11 11"/>',
    'chev_d': '<path d="M6 9l6 6 6-6"/>',
    'chev_u': '<path d="M6 15l6-6 6 6"/>',
    'chev_r': '<path d="M9 6l6 6-6 6"/>',
    'dice': '<rect x="4" y="4" width="16" height="16" rx="3.5"/><path d="M9 9h.01M15 9h.01M12 12h.01M9 15h.01M15 15h.01"/>',
    'heart': '<path d="M12 20s-7.5-4.6-7.5-10.2A4.2 4.2 0 0 1 12 7.3a4.2 4.2 0 0 1 7.5 2.5C19.5 15.4 12 20 12 20z"/>',
    'heart_broken': '<path d="M12 20s-7.5-4.6-7.5-10.2A4.2 4.2 0 0 1 12 7.3a4.2 4.2 0 0 1 7.5 2.5C19.5 15.4 12 20 12 20z"/><path d="M12 7.3l-1.5 4 3 2-1.5 3"/>',
    'hourglass': '<path d="M6.5 3.5h11M6.5 20.5h11M8 3.5v2.2c0 2 4 3.8 4 6.3s-4 4.3-4 6.3v2.2M16 3.5v2.2c0 2-4 3.8-4 6.3s4 4.3 4 6.3v2.2"/>',
    'bubble': '<path d="M5.5 4.5h13a2.5 2.5 0 0 1 2.5 2.5v7.5a2.5 2.5 0 0 1-2.5 2.5h-7l-4.5 3.5V17H5.5A2.5 2.5 0 0 1 3 14.5V7a2.5 2.5 0 0 1 2.5-2.5z"/>',
    'check': '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
    'target': '<circle cx="12" cy="12" r="7.5"/><circle cx="12" cy="12" r="2.5"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3"/>',
    'undo': '<path d="M9 14L4 9l5-5"/><path d="M4 9h10a6 6 0 0 1 0 12h-3"/>',
    'copy': '<rect x="8.5" y="8.5" width="12" height="12" rx="2.5"/><path d="M15.5 8.5V6A2.5 2.5 0 0 0 13 3.5H6A2.5 2.5 0 0 0 3.5 6v7A2.5 2.5 0 0 0 6 15.5h2.5"/>',
    'trash': '<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>',
    'rotate': '<path d="M20 12a8 8 0 1 1-2.4-5.7"/><path d="M20 4v5h-5"/>',
    'candle': '<path d="M9 21h6M10 11h4v10h-4z"/><path d="M12 3c2 2 2.2 4 0 5.5C9.8 7 10 5 12 3z"/>',
    'leaf': '<path d="M5 19C5 10 11 5 20 5c0 9-5 15-14 15"/><path d="M5 19l7-7"/>',
    'lamp': '<path d="M8.5 3.5h7l3 7h-13z"/><path d="M12 10.5v9M8 20.5h8"/>',
    'table': '<ellipse cx="12" cy="9" rx="9" ry="3.5"/><path d="M12 12.5V20M8 20.5h8"/>',
    'chair': '<path d="M7 3.5v17M7 12.5h10v8M7 12.5V3.5h8"/>',
    'counter': '<rect x="2.5" y="8" width="19" height="4" rx="1"/><path d="M4.5 12v8.5M19.5 12v8.5M4.5 16h15"/>',
    'stove': '<rect x="3.5" y="3.5" width="17" height="17" rx="2.5"/><circle cx="8.5" cy="8.5" r="2"/><circle cx="15.5" cy="8.5" r="2"/><path d="M3.5 13.5h17M8 17h.01M12 17h.01M16 17h.01"/>',
    'machine': '<rect x="4" y="3.5" width="16" height="7" rx="2"/><path d="M6 10.5v10h12v-10M10 10.5v3h4v-3"/><path d="M9.5 17.5h5"/>',
    'neon': '<rect x="3" y="6" width="18" height="12" rx="3"/><path d="M7 14.5l2-5 2 5M13 9.5v5l4-5v5"/>',
    'exit': '<path d="M10 4H5.5v16H10"/><path d="M15 8l4 4-4 4M19 12H9.5"/>',
    'book': '<path d="M4 19.5V5.5A2 2 0 0 1 6 3.5h14v14H6a2 2 0 0 0-2 2zm0 0A2 2 0 0 0 6 21.5h14"/>',
    'cube': '<path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z"/><path d="M12 12l8-4.5M12 12v9M12 12L4 7.5"/>',
    'sliders': '<path d="M4 6h9M18 6h2M4 12h3M11 12h9M4 18h11M19 18h1"/><circle cx="15.5" cy="6" r="2"/><circle cx="9" cy="12" r="2"/><circle cx="17" cy="18" r="2"/>',
    'continue': '<circle cx="12" cy="12" r="9"/><path d="M10 8.5l5 3.5-5 3.5z"/>',
    'plus': '<path d="M12 5v14M5 12h14"/>',
    'cursor': '<path d="M5 3l14 7-6 1.5L10 18z"/>',
}


def icon(name, x, y, size=24, color='text', sw=None):
    s = f'<use href="#i-{name}" x="{x:g}" y="{y:g}" width="{size:g}" height="{size:g}" color="{col(color)}"'
    if sw:
        s += f' stroke-width="{sw}"'
    return s + '/>'


# ---------------------------------------------------------------- doc
DEFS = f'''
<filter id="sh" x="-20%" y="-20%" width="140%" height="160%"><feDropShadow dx="0" dy="6" stdDeviation="8" flood-color="#000" flood-opacity=".45"/></filter>
<filter id="sh-s" x="-20%" y="-30%" width="140%" height="180%"><feDropShadow dx="0" dy="2" stdDeviation="3" flood-color="#000" flood-opacity=".35"/></filter>
<filter id="blur3" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="3"/></filter>
<filter id="blur8" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="8"/></filter>
<filter id="glow" x="-50%" y="-80%" width="200%" height="260%"><feGaussianBlur stdDeviation="5" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
<linearGradient id="g-panel" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{C['panel_top']}"/><stop offset="1" stop-color="{C['panel']}"/></linearGradient>
<linearGradient id="g-accent" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#6366F1"/><stop offset="1" stop-color="#4F46E5"/></linearGradient>
<linearGradient id="g-shift" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#818CF8"/><stop offset="1" stop-color="#4F46E5"/></linearGradient>
<linearGradient id="g-night" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="{C['violet']}"/><stop offset="1" stop-color="{C['cyan']}"/></linearGradient>
<linearGradient id="g-dawn" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#FFC46B" stop-opacity="0"/><stop offset="1" stop-color="#FFC46B" stop-opacity=".45"/></linearGradient>
<linearGradient id="g-guests" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#818CF8" stop-opacity=".45"/><stop offset="1" stop-color="#818CF8" stop-opacity=".03"/></linearGradient>
<radialGradient id="g-lamp"><stop offset="0" stop-color="#FFC37A" stop-opacity=".55"/><stop offset=".55" stop-color="#FFC46B" stop-opacity=".18"/><stop offset="1" stop-color="#FFC46B" stop-opacity="0"/></radialGradient>
<radialGradient id="g-candle"><stop offset="0" stop-color="#FFC37A" stop-opacity=".7"/><stop offset=".6" stop-color="#FF9A3D" stop-opacity=".15"/><stop offset="1" stop-color="#FF9A3D" stop-opacity="0"/></radialGradient>
<radialGradient id="g-hole"><stop offset="0" stop-color="#000"/><stop offset=".6" stop-color="#555"/><stop offset="1" stop-color="#fff"/></radialGradient>
<radialGradient id="g-table" cx=".4" cy=".35"><stop offset="0" stop-color="#B98457"/><stop offset="1" stop-color="#8C5D3C"/></radialGradient>
<radialGradient id="g-neon"><stop offset="0" stop-color="#6366F1" stop-opacity=".55"/><stop offset="1" stop-color="#6366F1" stop-opacity="0"/></radialGradient>
<linearGradient id="g-win" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#3C5591"/><stop offset="1" stop-color="#22305A"/></linearGradient>
<linearGradient id="g-winlight" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#8EA6FF" stop-opacity=".22"/><stop offset="1" stop-color="#8EA6FF" stop-opacity="0"/></linearGradient>
<linearGradient id="g-counter" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#C79A6B"/><stop offset=".45" stop-color="#A9784E"/><stop offset=".5" stop-color="#5E3A27"/><stop offset="1" stop-color="#4A2D1F"/></linearGradient>
<linearGradient id="g-sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#0A0918"/><stop offset=".6" stop-color="#1A1840"/><stop offset="1" stop-color="#2A2357"/></linearGradient>
<linearGradient id="g-facade" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2A2748"/><stop offset="1" stop-color="#1C1A35"/></linearGradient>
<linearGradient id="g-warmwin" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#FFC77F"/><stop offset="1" stop-color="#E8893A"/></linearGradient>
<pattern id="p-planks" width="192" height="48" patternUnits="userSpaceOnUse">
 <rect width="192" height="48" fill="#6A432F"/>
 <rect y="0" width="192" height="12" fill="#704732"/><rect y="24" width="192" height="12" fill="#734A34"/>
 <rect y="11" width="192" height="1" fill="#4E3022"/><rect y="23" width="192" height="1" fill="#4E3022"/><rect y="35" width="192" height="1" fill="#4E3022"/><rect y="47" width="192" height="1" fill="#4E3022"/>
 <rect x="40" y="0" width="1" height="12" fill="#4E3022"/><rect x="136" y="0" width="1" height="12" fill="#4E3022"/>
 <rect x="88" y="12" width="1" height="12" fill="#4E3022"/><rect x="184" y="12" width="1" height="12" fill="#4E3022"/>
 <rect x="20" y="24" width="1" height="12" fill="#4E3022"/><rect x="116" y="24" width="1" height="12" fill="#4E3022"/>
 <rect x="64" y="36" width="1" height="12" fill="#4E3022"/><rect x="160" y="36" width="1" height="12" fill="#4E3022"/>
</pattern>
<pattern id="p-tiles" width="32" height="32" patternUnits="userSpaceOnUse">
 <rect width="32" height="32" fill="#262C47"/><rect x="1" y="1" width="14" height="14" rx="1.5" fill="#323A5E"/><rect x="17" y="17" width="14" height="14" rx="1.5" fill="#323A5E"/><rect x="17" y="1" width="14" height="14" rx="1.5" fill="#2C3354"/><rect x="1" y="17" width="14" height="14" rx="1.5" fill="#2C3354"/>
</pattern>
<pattern id="p-hatch" width="6" height="6" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="2" height="6" fill="#F87171" fill-opacity=".35"/></pattern>
<pattern id="p-grid" width="48" height="48" patternUnits="userSpaceOnUse"><path d="M48 0H0V48" fill="none" stroke="#fff" stroke-opacity=".09"/></pattern>
'''


def icon_defs():
    out = []
    for k, v in ICONS.items():
        out.append(f'<symbol id="i-{k}" viewBox="0 0 24 24"><g fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">{v}</g></symbol>')
    return '\n'.join(out)


class Doc:
    def __init__(self, name, title):
        self.name, self.title, self.p, self.extra_defs = name, title, [], []

    def add(self, *s):
        self.p.extend(s)

    def note(self, s):
        self.p.append(f'<!-- {escape(s).replace("--", "—")} -->')

    def save(self):
        css = (f'text{{font-family:{FONT_UI};font-variant-numeric:tabular-nums}}'
               f'.h{{font-family:{FONT_HEAD}}}')
        svg = [f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W}" height="{H}">',
               f'<title>{escape(self.title)}</title>',
               f'<style>{css}</style>',
               '<defs>', DEFS, icon_defs(), *self.extra_defs, '</defs>',
               *self.p, '</svg>']
        path = os.path.join(OUT, self.name + '.svg')
        with open(path, 'w', encoding='utf-8') as f:
            f.write('\n'.join(svg))
        print('wrote', path)


# ---------------------------------------------------------------- primitives
def T(x, y, s, size=13, color='text', anchor='start', weight=400, cls=None, op=None, ls=None):
    a = f'x="{x:g}" y="{y:g}" font-size="{size}" fill="{col(color)}"'
    if anchor != 'start':
        a += f' text-anchor="{anchor}"'
    if weight != 400:
        a += f' font-weight="{weight}"'
    if cls:
        a += f' class="{cls}"'
    if op is not None:
        a += f' fill-opacity="{op}"'
    if ls:
        a += f' letter-spacing="{ls}"'
    return f'<text {a}>{escape(s)}</text>'


def R(x, y, w, h, fill, rx=0, op=None, stroke=None, sw=1, extra=''):
    a = f'x="{x:g}" y="{y:g}" width="{w:g}" height="{h:g}"'
    if rx:
        a += f' rx="{rx:g}"'
    a += f' fill="{col(fill)}"'
    if op is not None:
        a += f' fill-opacity="{op}"'
    if stroke:
        a += f' stroke="{col(stroke)}" stroke-width="{sw}"'
    return f'<rect {a} {extra}/>'


def panel(x, y, w, h, rx=10, op=.92, shadow=True):
    f = ' filter="url(#sh)"' if shadow else ''
    return (f'<g{f}><rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{rx}" fill="url(#g-panel)" fill-opacity="{op}"/></g>'
            f'<rect x="{x+.5}" y="{y+.5}" width="{w-1}" height="{h-1}" rx="{rx-.5}" fill="none" stroke="#fff" stroke-opacity=".07"/>')


def keycap(x, y, s, h=18):
    w = max(18, tw(s, 11, True) + 10)
    return (R(x, y, w, h, 'bg0', rx=4, op=.7, stroke='line') +
            T(x + w / 2, y + h - 5, s, 11, 'text2', 'middle', 600)), w


BTN = {
    'normal': ('raised', None, 'text'),
    'hover': ('hover', None, 'text'),
    'pressed': ('#1A1933', None, 'text2'),
    'active': ('url(#g-accent)', None, '#fff'),
    'primary': ('url(#g-accent)', None, '#fff'),
    'ghost': ('none', None, 'text2'),
    'disabled': ('raised', None, 'muted'),
    'danger': ('raised', 'bad', 'bad'),
}


def button(x, y, w, h, label='', state='normal', ic=None, key=None, size=13, align='middle', icol=None, rx=6):
    fill, stroke, tcol = BTN[state]
    out = []
    op = .45 if state == 'disabled' else None
    if fill != 'none':
        out.append(R(x, y, w, h, fill, rx=rx, op=op, stroke=stroke, sw=1))
    if state in ('normal', 'hover'):
        out.append(R(x + .5, y + .5, w - 1, h - 1, 'none', rx=rx - .5, stroke='#fff', extra='stroke-opacity=".06"'))
    if state == 'pressed':
        out.append(R(x, y, w, 2, '#000', rx=rx, op=.25))
    dy = 1 if state == 'pressed' else 0
    kw = 0
    if key:
        kc, kw = keycap(0, 0, key)
        kw += 6
    isz = 18 if h >= 28 else 16
    if label:
        lw = tw(label, size, True)
        if align == 'middle':
            content = (isz + 6 if ic else 0) + lw
            cx = x + (w - kw - content) / 2
        else:
            cx = x + 10
        if ic:
            out.append(icon(ic, cx, y + (h - isz) / 2 + dy, isz, icol or tcol))
            cx += isz + 6
        out.append(T(cx, y + h / 2 + size * .36 + dy, label, size, tcol, weight=600))
    elif ic:
        out.append(icon(ic, x + (w - isz) / 2, y + (h - isz) / 2 + dy, isz, icol or tcol))
    if key:
        kc, kw2 = keycap(x + w - kw2_calc(key) - 6, y + (h - 18) / 2, key)
        out.append(kc)
    return ''.join(out)


def kw2_calc(key):
    return max(18, tw(key, 11, True) + 10)


def bar(x, y, w, h, v, color, track='#fff', top=.08):
    return (R(x, y, w, h, track, rx=h / 2, op=top) +
            R(x, y, max(h, w * v), h, color, rx=h / 2))


def need_col(v):
    return 'good' if v >= .5 else ('warn' if v >= .25 else 'bad')


def chip(x, y, label, tone='neutral', ic=None, h=22, size=12):
    fills = {'neutral': ('#fff', .08, 'text'), 'good': ('good', .16, 'good'),
             'bad': ('bad', .16, 'bad'), 'warn': ('warn', .16, 'warn'),
             'accent': ('accent', .18, 'accent_hi'), 'info': ('cyan', .14, 'cyan')}
    f, o, tc = fills[tone]
    w = tw(label, size) + 20 + (20 if ic else 0)
    out = R(x, y, w, h, f, rx=h / 2, op=o)
    tx = x + 10
    if ic:
        out += icon(ic, tx, y + (h - 14) / 2, 14, tc)
        tx += 20
    out += T(tx, y + h / 2 + size * .36, label, size, tc, weight=500)
    return out, w


def section(x, y, label):
    return T(x, y, label.upper(), 11, 'muted', weight=700, ls='.6')


def close_btn(x, y):
    return R(x, y, 24, 24, '#fff', rx=6, op=.06) + icon('close', x + 4, y + 4, 16, 'text2')


def divider(x, y, w):
    return R(x, y, w, 1, '#fff', op=.07)


# ---------------------------------------------------------------- characters
def person(x, y, shirt, hair, skin='skin1', apron=None, rot=0, sc=1.0, sel=False):
    """Strict top-down view (historical v2 look): capsule shoulders, head, hands.
    No outline, only a darker edge of the same colour."""
    s = f'<g transform="translate({x:g},{y:g}) rotate({rot}) scale({sc})">'
    if sel:
        s += '<ellipse cx="0" cy="2" rx="22" ry="22" fill="none" stroke="#6366F1" stroke-width="2" stroke-dasharray="5 4"/>'
    s += '<ellipse cx="2" cy="6" rx="17" ry="13" fill="#000" opacity=".38" filter="url(#blur3)"/>'
    s += f'<rect x="-15" y="-9" width="30" height="19" rx="9.5" fill="{col(shirt)}" stroke="#000" stroke-opacity=".28" stroke-width="1.5"/>'
    if apron:
        s += f'<path d="M-7 -8.5L-5 9M7 -8.5L5 9" stroke="{col(apron)}" stroke-width="3" stroke-linecap="round"/>'
        s += f'<rect x="-9" y="5" width="18" height="6" rx="3" fill="{col(apron)}"/>'
    sk = col(skin)
    s += f'<circle cx="-13.5" cy="7" r="4" fill="{sk}" stroke="#000" stroke-opacity=".22"/><circle cx="13.5" cy="7" r="4" fill="{sk}" stroke="#000" stroke-opacity=".22"/>'
    s += f'<circle cx="0" cy="1.5" r="10" fill="{sk}" stroke="#000" stroke-opacity=".25" stroke-width="1.2"/>'
    s += f'<circle cx="0" cy="-1.8" r="9.3" fill="{col(hair)}"/>'
    s += '<ellipse cx="-3" cy="-5" rx="3.5" ry="2" fill="#fff" opacity=".12"/>'
    return s + '</g>'


def portrait(x, y, size, shirt, hair, skin='skin1', rx=8, ring=None):
    """Front-facing bust portrait (for lists)."""
    k = size / 40
    g = f'<g transform="translate({x:g},{y:g}) scale({k:g})">'
    g += f'<clipPath id="cp{int(x)}_{int(y)}"><rect width="40" height="40" rx="{rx / k:g}"/></clipPath>'
    g += f'<g clip-path="url(#cp{int(x)}_{int(y)})"><rect width="40" height="40" fill="#2C2950"/>'
    g += '<rect width="40" height="40" fill="#fff" opacity=".03"/>'
    g += f'<path d="M4 44c0-9 7-15 16-15s16 6 16 15z" fill="{col(shirt)}"/>'
    g += f'<rect x="16.5" y="24" width="7" height="7" rx="3" fill="{col(skin)}"/>'
    g += f'<ellipse cx="20" cy="18" rx="8.5" ry="9.5" fill="{col(skin)}"/>'
    g += f'<path d="M11 18c-1-7 3-11 9-11s10 4 9 11c-2-4-5-5.5-9-5.5S13 14 11 18z" fill="{col(hair)}"/>'
    g += '<circle cx="16.8" cy="19" r="1.1" fill="#2A1E1A"/><circle cx="23.2" cy="19" r="1.1" fill="#2A1E1A"/>'
    g += '</g>'
    if ring:
        g += f'<rect x="1" y="1" width="38" height="38" rx="{rx / k - 1:g}" fill="none" stroke="{col(ring)}" stroke-width="{2 / k:g}"/>'
    return g + '</g>'


# ---------------------------------------------------------------- furniture
def round_table(x, y, items=True):
    s = f'<ellipse cx="{x+3}" cy="{y+6}" rx="25" ry="23" fill="#000" opacity=".4" filter="url(#blur8)"/>'
    s += f'<circle cx="{x}" cy="{y}" r="22" fill="url(#g-table)" stroke="#5C3A26" stroke-width="2"/>'
    if items:
        s += f'<circle cx="{x-7}" cy="{y-4}" r="5" fill="#F4EEE6"/><circle cx="{x-7}" cy="{y-4}" r="3" fill="#5A3222"/>'
        s += f'<ellipse cx="{x+7}" cy="{y+5}" rx="7" ry="6" fill="#F4EEE6"/><ellipse cx="{x+7}" cy="{y+5}" rx="4" ry="3" fill="#E8A65A"/>'
    return s


def chair(x, y, rot=0):
    return (f'<g transform="translate({x},{y}) rotate({rot})">'
            '<rect x="-10" y="-8" width="22" height="22" rx="6" fill="#000" opacity=".35" filter="url(#blur3)"/>'
            '<rect x="-11" y="-11" width="22" height="22" rx="6" fill="#5E3B29"/>'
            '<rect x="-11" y="-11" width="22" height="6" rx="3" fill="#43291C"/>'
            '<rect x="-8" y="-3" width="16" height="11" rx="4" fill="#6E4731"/></g>')


def plant(x, y, r=16):
    s = f'<circle cx="{x+3}" cy="{y+5}" r="{r}" fill="#000" opacity=".35" filter="url(#blur3)"/>'
    s += f'<circle cx="{x}" cy="{y}" r="{r*.62:g}" fill="#7A4E35"/>'
    for i, a in enumerate(range(0, 360, 45)):
        dx, dy = math.cos(math.radians(a)) * r * .55, math.sin(math.radians(a)) * r * .55
        c = '#3E9C67' if i % 2 else '#2F7D52'
        s += f'<ellipse cx="{x+dx:.1f}" cy="{y+dy:.1f}" rx="{r*.5:g}" ry="{r*.26:g}" fill="{c}" transform="rotate({a} {x+dx:.1f} {y+dy:.1f})"/>'
    s += f'<circle cx="{x}" cy="{y}" r="{r*.25:g}" fill="#57B67D"/>'
    return s


def coffee_machine(x, y, ghost=False):
    op = ' opacity=".55"' if ghost else ''
    s = f'<g{op}>'
    s += f'<rect x="{x+2}" y="{y+6}" width="60" height="40" rx="7" fill="#000" opacity=".4" filter="url(#blur3)"/>'
    s += f'<rect x="{x}" y="{y}" width="60" height="40" rx="7" fill="#2C2F3E"/>'
    s += f'<rect x="{x+4}" y="{y+4}" width="52" height="14" rx="4" fill="#8E96AD"/>'
    s += f'<rect x="{x+4}" y="{y+4}" width="52" height="4" rx="2" fill="#fff" opacity=".35"/>'
    s += f'<circle cx="{x+50}" cy="{y+28}" r="3" fill="#FFC46B"/><circle cx="{x+50}" cy="{y+28}" r="6" fill="#FFC46B" opacity=".25"/>'
    s += f'<rect x="{x+14}" y="{y+22}" width="14" height="10" rx="3" fill="#B9C0D3"/>'
    s += f'<circle cx="{x+21}" cy="{y+35}" r="3.5" fill="#F4EEE6"/>'
    return s + '</g>'


def register(x, y):
    return (f'<rect x="{x+2}" y="{y+5}" width="34" height="28" rx="5" fill="#000" opacity=".4" filter="url(#blur3)"/>'
            f'<rect x="{x}" y="{y}" width="34" height="28" rx="5" fill="#3A3D55"/>'
            f'<rect x="{x+5}" y="{y+4}" width="24" height="9" rx="2" fill="#818CF8" opacity=".85"/>'
            f'<rect x="{x+5}" y="{y+16}" width="24" height="8" rx="2" fill="#2A2C40"/>')


def pastry_case(x, y, w=72):
    s = f'<rect x="{x}" y="{y}" width="{w}" height="26" rx="5" fill="#B4D2FF" fill-opacity=".16" stroke="#fff" stroke-opacity=".3"/>'
    for i in range(4):
        s += f'<ellipse cx="{x+12+i*16}" cy="{y+14}" rx="6" ry="4.5" fill="{["#E8A65A","#C97A3A","#F2D3A2","#E8A65A"][i]}"/>'
    s += f'<rect x="{x+3}" y="{y+3}" width="{w-6}" height="4" rx="2" fill="#fff" opacity=".18"/>'
    return s


def stove(x, y, ghost=False):
    s = f'<rect x="{x+3}" y="{y+6}" width="104" height="60" rx="8" fill="#000" opacity=".4" filter="url(#blur3)"/>'
    s += f'<rect x="{x}" y="{y}" width="104" height="60" rx="8" fill="#2A2C3A"/>'
    for cx in (26, 78):
        for cy in (20,):
            s += f'<circle cx="{x+cx}" cy="{y+cy+6}" r="14" fill="#1B1C27"/>'
            s += f'<circle cx="{x+cx}" cy="{y+cy+6}" r="10" fill="none" stroke="#FF6A3D" stroke-width="2" opacity=".85"/>'
            s += f'<circle cx="{x+cx}" cy="{y+cy+6}" r="16" fill="#FF6A3D" opacity=".10"/>'
    s += f'<rect x="{x+10}" y="{y+50}" width="84" height="4" rx="2" fill="#4A4D63"/>'
    return s


def sofa(x, y, w, h):
    s = f'<rect x="{x+3}" y="{y+5}" width="{w}" height="{h}" rx="10" fill="#000" opacity=".4" filter="url(#blur8)"/>'
    s += f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="10" fill="#6B2A52"/>'
    s += f'<rect x="{x}" y="{y}" width="12" height="{h}" rx="6" fill="#4F1E3C"/>'
    n = 3
    ch = (h - 16) / n
    for i in range(n):
        s += f'<rect x="{x+15}" y="{y+8+i*ch:.1f}" width="{w-20}" height="{ch-4:.1f}" rx="6" fill="#86356A"/>'
    return s


# ---------------------------------------------------------------- the café map
TABLES = [(220, 330), (340, 330), (220, 470), (340, 470), (500, 430), (640, 540), (760, 470)]
LAMPS = TABLES + [(680, 200), (580, 200)]


def rain_path(rng, regions, n):
    d = []
    for _ in range(n):
        x0, y0, x1, y1 = rng.choice(regions)
        x, y = rng.uniform(x0, x1), rng.uniform(y0, y1)
        ln = rng.uniform(10, 18)
        d.append(f'M{x:.0f} {y:.0f}l-3.5 {ln:.0f}')
    return '<path d="' + ''.join(d) + '" stroke="#A9BBFF" stroke-opacity=".22" stroke-width="1.2" stroke-linecap="round"/>'


def cafe_map(doc, mode='night', build=False):
    """mode: night | blackout | dim"""
    rng = random.Random(4)
    o = []
    o.append(R(0, 0, W, H, '#0D0E20'))
    # platform + rails (top), street (bottom)
    o.append(R(0, 0, W, 64, '#141632'))
    for x in range(0, W, 26):
        o.append(R(x, 14, 10, 34, '#2A2238'))
    o.append(R(0, 20, W, 3, '#5B5F80'))
    o.append(R(0, 40, W, 3, '#5B5F80'))
    o.append(R(0, 64, W, 30, '#1E2140'))
    o.append(R(0, 64, W, 3, '#FFCC4D', op=.35))  # platform edge
    o.append(R(0, 690, W, 30, '#1A1C38'))
    # puddles
    for (px, py, pw) in ((150, 76, 90), (980, 80, 120), (420, 700, 140)):
        o.append(f'<ellipse cx="{px}" cy="{py}" rx="{pw/2}" ry="6" fill="#3A4A86" opacity=".45"/>')
    o.append(f'<ellipse cx="800" cy="82" rx="46" ry="5" fill="#6366F1" opacity=".22"/>')
    # building shell
    o.append(R(96, 96, 1088, 594, '#24213F', rx=4))
    o.append(R(112, 112, 752, 562, 'url(#p-planks)'))
    o.append(R(880, 112, 288, 562, 'url(#p-tiles)'))
    o.append(R(864, 112, 16, 562, '#2E2A4E'))
    o.append(R(864, 360, 16, 72, 'url(#p-tiles)'))  # doorway into the kitchen
    o.append(R(112, 112, 752, 562, '#000', op=.08))
    # rug
    o.append(R(440, 380, 260, 220, '#3B2850', rx=14, op=.55))
    o.append(R(452, 392, 236, 196, 'none', rx=10, stroke='#8F5BB0', sw=1.5, extra='stroke-opacity=".35" stroke-dasharray="2 6"'))
    # windows on top wall + cold light
    for wx in (150, 300, 1000):
        o.append(f'<path d="M{wx} 112h110l50 230H{wx-50}z" fill="url(#g-winlight)"/>')
    for wx in (150, 300, 1000):
        o.append(R(wx, 96, 110, 16, 'url(#g-win)', rx=2))
        for i in range(7):
            xx = wx + 8 + i * 15 + rng.uniform(-3, 3)
            o.append(R(xx, 98, 1.2, rng.uniform(5, 11), '#C6D4FF', op=.5))
    # door to platform
    o.append(R(760, 96, 80, 16, '#3A2A20'))
    o.append(R(764, 99, 72, 10, '#FFC46B', rx=2, op=.55))
    # neon sign outside
    o.append('<ellipse cx="800" cy="80" rx="80" ry="22" fill="url(#g-neon)"/>' if mode != 'blackout' else '')
    o.append(R(744, 70, 112, 22, '#140F22', rx=5))
    neon_col = '#A5B4FC' if mode != 'blackout' else '#4A3550'
    filt = ' filter="url(#glow)"' if mode != 'blackout' else ''
    o.append(f'<text x="800" y="86.5" font-size="15" font-weight="800" class="h" fill="{neon_col}" text-anchor="middle" letter-spacing="2"{filt}>KAVÁRNA</text>')
    # back shelf + counter
    o.append(R(500, 112, 360, 22, '#4A2E20'))
    for i in range(16):
        c = ['#F4EEE6', '#3E9C67', '#818CF8', '#E8893A'][i % 4]
        o.append(f'<circle cx="{514+i*21}" cy="123" r="{4 if i%4==0 else 3.5}" fill="{c}" opacity=".85"/>')
    o.append(R(503, 258, 362, 44, '#000', rx=6, op=.45, extra='filter="url(#blur8)"'))
    o.append(R(500, 250, 360, 40, 'url(#g-counter)', rx=6))
    o.append(R(500, 250, 360, 4, '#fff', rx=2, op=.18))
    o.append(R(820, 290, 40, 80, 'url(#g-counter)', rx=6))
    o.append(coffee_machine(540, 222))
    o.append(pastry_case(640, 258))
    o.append(register(770, 246))
    o.append(f'<circle cx="618" cy="266" r="5" fill="#F4EEE6"/><circle cx="618" cy="266" r="3" fill="#5A3222"/>')
    # kitchen
    o.append(stove(910, 130))
    o.append(R(1090, 124, 64, 70, '#C3CBDD', rx=6))
    o.append(R(1090, 124, 64, 8, '#fff', rx=4, op=.4))
    o.append(R(1146, 150, 3, 24, '#7D8499', rx=1.5))
    o.append(R(910, 300, 200, 44, '#8D95AA', rx=6))
    o.append(R(910, 300, 200, 5, '#fff', rx=2, op=.3))
    o.append(R(1120, 300, 40, 44, '#6E768B', rx=6))
    o.append(f'<ellipse cx="1140" cy="322" rx="13" ry="14" fill="#3E4A66"/>')
    o.append(f'<circle cx="960" cy="322" r="9" fill="#E8A65A"/><circle cx="990" cy="318" r="6" fill="#F4EEE6"/>')
    # booth + tables + chairs
    o.append(sofa(116, 520, 44, 140))
    for i, (tx, ty) in enumerate(TABLES):
        o.append(chair(tx - 34, ty, -90))
        o.append(chair(tx + 34, ty, 90))
    for tx, ty in TABLES:
        o.append(round_table(tx, ty))
    o.append(R(176, 556, 40, 80, '#7A4E35', rx=8, op=0))
    o.append(round_table(196, 590))
    # plants
    o.append(plant(140, 145))
    o.append(plant(838, 650, 18))
    o.append(plant(470, 150, 14))
    # people: staff (apron = accent), guests
    people = []
    people.append(person(578, 205, '#3C6FA8', '#3A2418', 'skin1', apron='#6366F1', rot=0))          # Jana, barista
    people.append(person(800, 214, '#5E5AA8', '#C98B4A', 'skin2', apron='#6366F1', rot=0))          # Petr, register
    people.append(person(960, 220, '#2F7D52', '#1C1410', 'skin1', apron='#6366F1', rot=180))       # Tomáš, kitchen
    people.append(person(430, 400, '#4A6FB8', '#7A3B2A', 'skin3', apron='#6366F1', rot=40, sel=(mode == 'night')))  # Míša
    people.append(person(210, 630, '#8A6A3A', '#E8D4A8', 'skin1', apron='#6366F1', rot=-30))       # Olga
    # guests
    people.append(person(800, 330, '#6A4B8A', '#2A1A14', 'skin2', rot=180))
    people.append(person(806, 372, '#2E6B78', '#B5643A', 'skin1', rot=180))
    people.append(person(792, 414, '#8C3E4E', '#1E1612', 'skin3', rot=170))
    for (tx, ty), side, sh, hr, sk in [((220, 330), -1, '#3E7C8C', '#2A1A14', 'skin1'), ((220, 330), 1, '#A0567A', '#D9B070', 'skin2'),
                                       ((340, 330), -1, '#5A6E3A', '#1E1612', 'skin3'), ((340, 470), 1, '#7A5AA8', '#3A2418', 'skin1'),
                                       ((500, 430), -1, '#B07A3A', '#2A1A14', 'skin2'), ((500, 430), 1, '#3A5A9A', '#8A4A2A', 'skin1'),
                                       ((640, 540), -1, '#9A3A5A', '#1C1410', 'skin3'), ((760, 470), 1, '#4A8A7A', '#C98B4A', 'skin1')]:
        people.append(person(tx + side * 38, ty, sh, hr, sk, rot=-90 * side))
    o.extend(people)
    # dog near table 4
    o.append('<g transform="translate(470 470) rotate(20)"><ellipse cx="2" cy="4" rx="16" ry="9" fill="#000" opacity=".35" filter="url(#blur3)"/>'
             '<ellipse cx="0" cy="0" rx="14" ry="8" fill="#C98B4A"/><circle cx="14" cy="-1" r="6.5" fill="#D9A05E"/>'
             '<ellipse cx="13" cy="-7" rx="3" ry="2" fill="#8A5A30"/><ellipse cx="13" cy="5" rx="3" ry="2" fill="#8A5A30"/>'
             '<path d="M-13 0l-7 -3" stroke="#C98B4A" stroke-width="3" stroke-linecap="round"/></g>')
    # job progress bubble over barista
    if mode == 'night':
        o.append('<g transform="translate(578 168)"><circle r="13" fill="#17162E" fill-opacity=".92"/>'
                 '<circle r="11" fill="none" stroke="#fff" stroke-opacity=".12" stroke-width="2.5"/>'
                 '<circle r="11" fill="none" stroke="#818CF8" stroke-width="2.5" stroke-dasharray="48 100" transform="rotate(-90)" stroke-linecap="round"/>'
                 f'{icon("cup", -8, -8, 16, "text")}</g>')
    # lighting
    mid = f'm-{doc.name}'
    holes = LAMPS if mode != 'blackout' else TABLES[:5]
    hr = 150 if mode != 'blackout' else 70
    dark = {'night': .38, 'blackout': .80, 'dim': .55}[mode]
    m = [f'<mask id="{mid}"><rect width="{W}" height="{H}" fill="#fff"/>']
    for (lx, ly) in holes:
        m.append(f'<circle cx="{lx}" cy="{ly}" r="{hr}" fill="url(#g-hole)"/>')
    if mode != 'blackout':
        m.append(f'<circle cx="800" cy="80" r="90" fill="url(#g-hole)"/>')
    m.append('</mask>')
    doc.extra_defs.append(''.join(m))
    o.append(f'<rect width="{W}" height="{H}" fill="#07061A" opacity="{dark}" mask="url(#{mid})"/>')
    if mode != 'blackout':
        for (lx, ly) in LAMPS:
            o.append(f'<circle cx="{lx}" cy="{ly}" r="130" fill="url(#g-lamp)" style="mix-blend-mode:screen" opacity=".55"/>')
    else:
        for (lx, ly) in TABLES[:5]:
            o.append(f'<circle cx="{lx}" cy="{ly}" r="70" fill="url(#g-candle)" style="mix-blend-mode:screen"/>')
            o.append(f'<rect x="{lx-2}" y="{ly-14}" width="4" height="8" rx="1.5" fill="#F4EEE6"/><ellipse cx="{lx}" cy="{ly-17}" rx="2.2" ry="3.5" fill="#FFC37A"/>')
    # rain (exterior only)
    o.append(rain_path(rng, [(0, 0, W, 94), (0, 690, W, H), (0, 94, 96, 690), (1184, 94, W, 690)], 340))
    if build:
        o.append(R(112, 112, 1056, 562, 'url(#p-grid)'))
    doc.add(*o)


# ---------------------------------------------------------------- HUD
STAFF = [
    # name, short, role, job, E, M, H, shirt, hair, skin
    ('Jana Dvořáková', 'Jana D.', 'Barista', 'making a cappuccino', .72, .85, .60, '#3C6FA8', '#3A2418', 'skin1'),
    ('Tomáš Říha', 'Tomáš Ř.', 'Cook', 'frying potato pancakes', .45, .62, .30, '#2F7D52', '#1C1410', 'skin1'),
    ('Petr Šťastný', 'Petr Š.', 'Cashier', 'taking an order', .80, .40, .75, '#5E5AA8', '#C98B4A', 'skin2'),
    ('Míša Krejčí', 'Míša K.', 'Server', 'carrying an order → table 4', .22, .55, .50, '#4A6FB8', '#7A3B2A', 'skin3'),
    ('Olga Ševčenko', 'Olga Š.', 'Cleaning', 'mopping by the door', .66, .90, .82, '#8A6A3A', '#E8D4A8', 'skin1'),
    ('Ondřej Bárta', 'Ondřej B.', 'Barista', 'on a break · kitchen', .15, .30, .20, '#5A4A7A', '#2A1A14', 'skin2'),
    ('Kája Nováková', 'Kája N.', 'Cook', 'off shift', .95, .70, .65, '#9A3A5A', '#D9B070', 'skin1'),
]


def hud_clock(doc):
    x, y, w, h = 12, 12, 340, 64
    doc.note('HUD: clock + speed (anchor: top left, 12/12)')
    doc.add(panel(x, y, w, h))
    doc.add(icon('clock', 24, 24, 20, 'cyan'))
    doc.add(T(50, 42, '01:40', 22, 'text', weight=700, cls='h'))
    doc.add(T(118, 41, 'Fri · night 3', 12, 'text2'))
    labels = [('pause', 'normal'), ('play', 'active'), ('ff2', 'normal'), ('ff3', 'normal')]
    for i, (ic, st) in enumerate(labels):
        doc.add(button(222 + i * 30, 20, 28, 28, '', st, ic=ic, rx=6))
    # night progress 22:00 → 06:00
    bx, by, bw = 24, 58, 316
    doc.add(R(bx, by, bw, 6, '#fff', rx=3, op=.08))
    doc.add(R(bx + bw * 7 / 8, by, bw / 8, 6, 'url(#g-dawn)', rx=3))
    p = 3 + 40 / 60
    doc.add(R(bx, by, bw * p / 8, 6, 'url(#g-night)', rx=3))
    for i in range(1, 8):
        doc.add(R(bx + bw * i / 8 - .5, by + 1, 1, 4, '#000', op=.35))
    for t in (47 / 60, 2 + 15 / 60, 6 + 52 / 60):
        tx = bx + bw * t / 8
        doc.add(f'<path d="M{tx-3:.1f} {by-6}h6l-3 4z" fill="{C["amber"]}"/>')
    cx = bx + bw * p / 8
    doc.add(f'<circle cx="{cx:.1f}" cy="{by+3}" r="5" fill="#fff" filter="url(#sh-s)"/>')


def hud_ticker(doc, text='A couple came in with a dog', t='01:32', tone='info', ic='info'):
    doc.note('HUD: event ticker (anchor: top centre, w 400)')
    x, y, w, h = 440, 12, 400, 32
    doc.add(panel(x, y, w, h, rx=8))
    c = {'info': 'cyan', 'bad': 'bad', 'warn': 'warn'}[tone]
    doc.add(icon(ic, x + 12, y + 8, 16, c))
    doc.add(T(x + 36, y + 21, t, 12, 'muted', weight=600))
    doc.add(T(x + 76, y + 21, text, 13, 'text' if tone == 'info' else c, weight=500 if tone == 'info' else 600))
    doc.add(R(x + w - 44, y + 7, 1, 18, '#fff', op=.08))
    doc.add(icon('log', x + w - 34, y + 8, 16, 'text2'))
    doc.add(T(x + w - 14, y + 21, 'L', 11, 'muted', 'end', 700))


def hud_resources(doc):
    doc.note('HUD: resources (anchor: top right, 12/12)')
    x, y, w, h = 928, 12, 340, 64
    doc.add(panel(x, y, w, h))
    doc.add(icon('coin', 942, 23, 18, 'amber'))
    doc.add(T(966, 38, '12 480 Kč', 16, 'amber', weight=700))
    doc.add(icon('star', 1068, 23, 18, 'warn'))
    doc.add(T(1092, 38, '4,2', 15, 'text', weight=700))
    doc.add(T(1118, 38, '+0,1', 11, 'good', weight=600))
    doc.add(icon('guest', 1162, 23, 18, 'text2'))
    doc.add(T(1186, 38, '14 / 20', 15, 'text', weight=700))
    doc.add(divider(940, 48, 316))
    doc.add(T(942, 66, '+1 860 Kč tonight', 12, 'good', weight=500))
    doc.add(T(1058, 66, '·  queue 3', 12, 'text2'))
    doc.add(icon('rain', 1150, 54, 14, 'text2'))
    doc.add(T(1168, 66, 'rain, 9 °C', 12, 'text2'))


def hud_roster(doc, sel=3):
    doc.note('HUD: crew on shift (anchor: left edge, y 88)')
    on = [0, 1, 2, 3, 4]
    x, y = 12, 88
    doc.add(panel(x, y, 56, 12 + len(on) * 52, rx=10))
    for i, si in enumerate(on):
        s = STAFF[si]
        py = y + 8 + i * 52
        ring = 'accent' if si == sel else None
        doc.add(portrait(20, py, 40, s[7], s[8], s[9], rx=8, ring=ring))
        worst = min(s[4], s[5], s[6])
        doc.add(bar(20, py + 44, 40, 3, worst, need_col(worst)))
        if worst < .25:
            doc.add(f'<circle cx="57" cy="{py+3}" r="5" fill="{C["bad"]}" stroke="#17162E" stroke-width="2"/>')


TOOLBAR = [('staff', 'Crew', 'C'), ('prio', 'Priorities', 'P'), ('calendar', 'Shiftella', 'T'),
           ('hammer', 'Build', 'B'), ('log', 'Log', 'L')]


def hud_toolbar(doc, active=None):
    doc.note('HUD: screen toolbar (anchor: bottom left, 12/12)')
    widths = [8 + 18 + 6 + tw(lbl, 13, True) + 10 + 18 + 6 for _, lbl, _ in TOOLBAR]
    total = sum(widths) + 4 * (len(TOOLBAR)) + 28 + 8
    x, y = 12, 672
    doc.add(panel(x, y, total, 36, rx=10))
    cx = x + 4
    for (ic, lbl, k), bw in zip(TOOLBAR, widths):
        st = 'active' if active == k else 'ghost'
        doc.add(button(cx, y + 4, bw, 28, lbl, st, ic=ic, key=k, icol=None if st == 'active' else 'text2'))
        cx += bw + 4
    doc.add(button(cx, y + 4, 28, 28, '', 'ghost', ic='menu', icol='text2'))
    return total


def toast(doc, x, y, title, sub, ic, tone, prog=None, w=320):
    c = {'info': 'cyan', 'bad': 'bad', 'warn': 'warn', 'good': 'good'}[tone]
    doc.add(panel(x, y, w, 56, rx=10))
    doc.add(f'<rect x="{x}" y="{y+10}" width="3" height="36" rx="1.5" fill="{col(c)}"/>')
    doc.add(f'<circle cx="{x+30}" cy="{y+28}" r="16" fill="{col(c)}" fill-opacity=".14"/>')
    doc.add(icon(ic, x + 20, y + 18, 20, c))
    doc.add(T(x + 56, y + 24, title, 13, 'text', weight=600))
    doc.add(T(x + 56, y + 42, sub, 12, 'text2'))
    doc.add(T(x + w - 12, y + 24, 'More', 12, 'accent_hi', 'end', 600))
    if prog is not None:
        doc.add(R(x + 10, y + 52, w - 20, 2, '#fff', rx=1, op=.06))
        doc.add(R(x + 10, y + 52, (w - 20) * prog, 2, c, rx=1, op=.7))


def base_hud(doc, roster=True, active=None, ticker=None):
    hud_clock(doc)
    if ticker:
        hud_ticker(doc, **ticker)
    else:
        hud_ticker(doc)
    hud_resources(doc)
    if roster:
        hud_roster(doc)
    return hud_toolbar(doc, active)


# ================================================================ SCREENS
def screen_hud():
    d = Doc('hud', 'Shiftella — HUD')
    cafe_map(d)
    # floating name tag over selected staff
    d.add(R(398, 362, 64, 20, '#17162E', rx=10, op=.9))
    d.add(T(430, 376, 'Míša', 12, 'text', 'middle', 600))
    d.add(R(426, 382, 8, 3, 'bad', rx=1.5))
    base_hud(d)
    toast(d, 948, 652, 'A group with a guitar', '4 guests · they want to sing, it will be loud', 'guest', 'info', prog=.62)
    d.save()


def screen_staff():
    d = Doc('staff', 'Shiftella — Crew')
    cafe_map(d)
    base_hud(d, roster=False, active='C')
    x, y, w, h = 12, 88, 600, 572
    d.note('Crew panel (C): x12 y88 600×572, anchor left edge')
    d.add(panel(x, y, w, h, rx=12))
    d.add(T(28, 116, 'Crew', 17, 'text', weight=700, cls='h'))
    kc, kw = keycap(28 + tw('Crew', 17, True) * 1.12 + 8, 102, 'C')
    d.add(kc)
    d.add(T(28 + tw('Crew', 17, True) * 1.12 + 36, 115, '5 on shift · 7 in total', 12, 'text2'))
    d.add(close_btn(576, 100))
    d.add(divider(24, 132, 576))
    # list
    d.add(T(32, 152, 'CREW MEMBER', 11, 'muted', weight=700, ls='.6'))
    for ic, cx in (('bolt', 230), ('smile', 266), ('bowl', 302)):
        d.add(icon(ic, cx - 8, 139, 16, 'text2'))
    for i, s in enumerate(STAFF):
        ry = 160 + i * 48
        if i == 3:
            d.add(R(24, ry, 304, 44, 'accent', rx=8, op=.13))
            d.add(R(24, ry + 8, 3, 28, 'accent', rx=1.5))
        elif i == 1:
            d.add(R(24, ry, 304, 44, '#fff', rx=8, op=.05))
        d.add(portrait(34, ry + 6, 32, s[7], s[8], s[9], rx=7))
        off = s[3] == 'off shift'
        d.add(T(76, ry + 19, s[0], 13, 'text' if not off else 'text2', weight=600))
        dot = 'muted' if off else ('warn' if 'break' in s[3] else 'cyan')
        d.add(f'<circle cx="80" cy="{ry+32}" r="3" fill="{col(dot)}"/>')
        d.add(T(89, ry + 36, s[3], 12, 'text2'))
        for v, cx in ((s[4], 230), (s[5], 266), (s[6], 302)):
            d.add(bar(cx - 14, ry + 20, 28, 4, v, need_col(v)))
    d.add(T(32, 520, 'Sort by', 12, 'muted'))
    sx = 104
    for lbl, st in (('Name', 'normal'), ('Energy', 'active'), ('Mood', 'normal')):
        bw = tw(lbl, 12, True) + 20
        d.add(button(sx, 506, bw, 24, lbl, st, size=12))
        sx += bw + 4
    d.add(button(24, 620, 132, 28, 'Hire', 'normal', ic='plus', key='H'))
    d.add(T(166, 638, 'candidates: 3', 12, 'muted'))
    d.add(R(340, 140, 1, 508, '#fff', op=.07))
    # details
    s = STAFF[3]
    d.note('Card of the selected crew member')
    d.add(portrait(352, 144, 64, s[7], s[8], s[9], rx=12))
    d.add(T(428, 162, s[0], 16, 'text', weight=700, cls='h'))
    d.add(T(428, 181, 'Server · 22 years · 150 Kč/h', 12, 'text2'))
    c, cw = chip(428, 192, 'carrying an order → table 4', 'info', ic='cup')
    d.add(c)
    d.add(section(352, 236, 'Needs'))
    for i, (ic, lbl, v, txt) in enumerate((('bolt', 'Energy', s[4], '22 %'), ('smile', 'Mood', s[5], '55 %'), ('bowl', 'Hunger', s[6], '50 %'))):
        ry = 246 + i * 24
        cc = need_col(v)
        d.add(icon(ic, 352, ry, 16, cc if v < .25 else 'text2'))
        d.add(T(376, ry + 13, lbl, 13, 'text'))
        d.add(bar(466, ry + 5, 96, 6, v, cc))
        d.add(T(600, ry + 13, txt, 12, cc if v < .25 else 'text2', 'end', 600))
    d.add(section(352, 340, 'Traits'))
    cx, cy = 352, 350
    for lbl, tone in (('Night Owl', 'good'), ('Chatterbox', 'neutral'), ('Anxious', 'bad')):
        cw = tw(lbl, 12) + 20
        if cx + cw > 600:
            cx, cy = 352, cy + 28
        c, cw = chip(cx, cy, lbl, tone)
        d.add(c)
        cx += cw + 6
    d.add(section(352, 420, 'Skills'))
    for i, (lbl, v, fav) in enumerate((('Serving', 7, True), ('Register', 5, False), ('Bar', 3, False), ('Cleaning', 4, False), ('Kitchen', 1, False))):
        ry = 430 + i * 22
        d.add(T(352, ry + 14, lbl, 13, 'text'))
        if fav:
            d.add(icon('heart', 392, ry + 2, 14, 'accent_hi'))
        d.add(bar(440, ry + 6, 132, 6, v / 10, 'amber'))
        d.add(T(600, ry + 14, str(v), 13, 'text', 'end', 600))
    d.add(section(352, 560, 'Relationships'))
    for i, (who, rel, val, tone, sk) in enumerate((('Jana', 'friend', '+32', 'good', 0), ('Tomáš', 'annoying', '−18', 'bad', 1), ('Olga', 'colleague', '+5', 'text2', 4))):
        ry = 570 + i * 28
        t = STAFF[sk]
        d.add(portrait(352, ry, 22, t[7], t[8], t[9], rx=11))
        d.add(icon('heart' if tone != 'bad' else 'heart_broken', 382, ry + 4, 14, 'accent_hi' if tone == 'good' else ('bad' if tone == 'bad' else 'muted')))
        d.add(T(402, ry + 15, who, 13, 'text', weight=600))
        d.add(T(402 + tw(who, 13, True) + 8, ry + 15, rel, 12, 'text2'))
        d.add(T(600, ry + 15, val, 13, tone if tone != 'text2' else 'text2', 'end', 700))
    d.save()


def screen_priorities():
    d = Doc('priorities', 'Shiftella — Work priorities')
    cafe_map(d)
    base_hud(d, roster=False, active='P')
    x, y, w, h = 12, 88, 516, 420
    d.note('Priorities panel (P): x12 y88 516×420')
    d.add(panel(x, y, w, h, rx=12))
    d.add(T(28, 116, 'Work priorities', 17, 'text', weight=700, cls='h'))
    kc, kw = keycap(28 + tw('Work priorities', 17, True) * 1.12 + 8, 102, 'P')
    d.add(kc)
    d.add(close_btn(492, 100))
    d.add(divider(24, 132, 492))
    jobs = [('pan', 'Kitchen'), ('cup', 'Bar'), ('register', 'Register'), ('broom', 'Cleaning')]
    for i, (ic, lbl) in enumerate(jobs):
        x0 = 216 + i * 72
        if i == 2:
            d.add(R(x0 + 4, 140, 64, 52, '#fff', rx=8, op=.05))
        d.add(icon(ic, x0 + 24, 146, 24, 'text'))
        d.add(T(x0 + 36, 186, lbl, 12, 'text2', 'middle', 600))
    d.add(T(32, 186, 'CREW MEMBER', 11, 'muted', weight=700, ls='.6'))
    grid = [[3, 1, 2, 4], [1, 0, 4, 3], [4, 3, 1, 2], [-1, 2, 2, 3], [0, 4, 0, 1], [2, 1, 3, 0], [1, 0, 3, 4]]
    skill = [[1, 8, 5, 4], [9, 2, 3, 5], [2, 4, 8, 6], [0, 3, 5, 4], [3, 2, 2, 9], [4, 7, 4, 3], [7, 1, 4, 3]]
    for r, s in enumerate(STAFF):
        ry = 200 + r * 36
        if r % 2 == 0:
            d.add(R(24, ry, 492, 34, '#fff', rx=6, op=.025))
        d.add(portrait(30, ry + 5, 24, s[7], s[8], s[9], rx=6))
        d.add(T(62, ry + 22, s[0], 13, 'text', weight=500))
        for c in range(4):
            x0 = 216 + c * 72
            v = grid[r][c]
            cx, cy, cw, ch = x0 + 12, ry + 4, 48, 26
            if v == -1:
                d.add(R(cx, cy, cw, ch, 'url(#p-hatch)', rx=6))
                d.add(R(cx, cy, cw, ch, 'none', rx=6, stroke='bad', extra='stroke-opacity=".35"'))
                d.add(T(cx + cw / 2, cy + 17, '✕', 12, 'bad', 'middle', 700))
                continue
            fills = {1: ('url(#g-accent)', None, '#fff'), 2: ('amber', .9, '#2A1A0A'), 3: ('#fff', .14, 'text'), 4: ('#fff', .07, 'text2'), 0: ('none', None, 'muted')}
            f, o, tc = fills[v]
            if f != 'none':
                d.add(R(cx, cy, cw, ch, f, rx=6, op=o))
            else:
                d.add(R(cx + .5, cy + .5, cw - 1, ch - 1, 'none', rx=6, stroke='#fff', extra='stroke-opacity=".1" stroke-dasharray="3 3"'))
            d.add(T(cx + cw / 2, cy + 18, str(v) if v else '—', 14, tc, 'middle', 700))
            sk = skill[r][c] / 10
            d.add(R(cx + 8, cy + ch - 4, (cw - 16) * sk, 2, '#fff' if v in (1, 2) else 'cyan', rx=1, op=.55))
    # hover cell (Míša / Register)
    hx, hy = 216 + 2 * 72 + 12, 200 + 3 * 36 + 4
    d.add(R(hx - 2, hy - 2, 52, 30, 'none', rx=8, stroke='cyan', sw=2))
    tx, ty = hx + 60, hy - 8
    d.add(panel(tx, ty, 196, 52, rx=8))
    d.add(T(tx + 10, ty + 20, 'Míša · Register', 13, 'text', weight=600))
    d.add(T(tx + 10, ty + 38, 'skill 5 · wheel ↑↓ — change', 12, 'text2'))
    d.add(divider(24, 456, 492))
    d.add(T(28, 478, 'Left-click — higher  ·  right-click — lower  ·  wheel  ·  drag — paint', 12, 'text2'))
    d.add(T(28, 496, 'Shift + click a column — everyone  ·  1 = most important,  — = never', 12, 'muted'))
    d.save()


def screen_schedule():
    d = Doc('schedule', 'Shiftella — Rozpis (day phase)')
    cafe_map(d, mode='dim')
    d.add(R(0, 0, W, H, '#0B0A18', op=.55))
    d.note('Top bar of the planning phase 1280×48')
    d.add(panel(0, -10, W, 58, rx=0))
    d.add(T(24, 31, 'Planning · day 4', 17, 'text', weight=700, cls='h'))
    d.add(T(232, 30, 'next night: Friday, 22:00–06:00', 12, 'text2'))
    d.add(icon('coin', 1110, 15, 18, 'amber'))
    d.add(T(1134, 30, '12 480 Kč', 16, 'amber', weight=700))
    x, y, w, h = 24, 64, 1232, 520
    d.add(panel(x, y, w, h, rx=12))
    d.add(T(40, 94, 'Shift rozpis', 16, 'text', weight=700, cls='h'))
    kc, kw = keycap(40 + tw('Shift rozpis', 16, True) * 1.12 + 8, 80, 'T')
    d.add(kc)
    cx = 1240
    for lbl, ic, tone in reversed((('rain 80 %', 'rain', 'info'), ('trains 22:47 · 00:15 · 04:52', 'train', 'neutral'), ('forecast ~46 guests', 'guest', 'neutral'))):
        cw = tw(lbl, 12) + 40
        cx -= cw
        c, _ = chip(cx, 76, lbl, tone, ic=ic, h=24)
        d.add(c)
        cx -= 8
    d.add(divider(40, 110, 1200))
    GX, SW = 264, 44
    for hh in range(9):
        lab = f'{(22 + hh) % 24:02d}:00'
        d.add(T(GX + hh * 88, 128, lab, 11, 'text2', 'middle', 600))
    d.add(T(40, 128, 'CREW MEMBER', 11, 'muted', weight=700, ls='.6'))
    for lbl, cxx in (('Hours', 1010), ('Kč', 1090), ('Energy', 1190)):
        d.add(T(cxx, 128, lbl.upper(), 11, 'muted', 'middle', 700, ls='.6'))
    shifts = {0: [(0, 6), 'b6', (7, 10)], 1: [(0, 8), 'b8', (9, 16)], 2: [(2, 7), 'b7', (8, 12)], 3: [(4, 16)],
              4: [(8, 16)], 5: [(0, 8)], 6: []}
    hours = {0: 4.5, 1: 7.5, 2: 4.5, 3: 6, 4: 4, 5: 4, 6: 0}
    ROWY = 136
    for i, s in enumerate(STAFF):
        ry = ROWY + i * 40
        d.add(R(40, ry, 1200, 36, '#fff', rx=6, op=.035 if i % 2 == 0 else .0))
        d.add(portrait(46, ry + 6, 24, s[7], s[8], s[9], rx=6))
        d.add(T(80, ry + 17, s[0], 13, 'text', weight=600))
        d.add(T(80, ry + 31, s[2], 11, 'text2'))
        for hh in range(9):
            d.add(R(GX + hh * 88 - .5, ry + 2, 1, 32, '#fff', op=.07))
        for j in range(8):
            d.add(R(GX + j * 88 + 44 - .5, ry + 12, 1, 12, '#fff', op=.04))
        segs = shifts[i]
        for sg in segs:
            if isinstance(sg, str):
                b = int(sg[1:])
                bx = GX + b * SW
                d.add(R(bx + 2, ry + 7, SW - 4, 22, 'amber', rx=6, op=.22))
                d.add(icon('cup', bx + SW / 2 - 7, ry + 11, 14, 'amber'))
                continue
            a, b = sg
            sx, sw_ = GX + a * SW + 2, (b - a) * SW - 4
            warn = i == 3
            d.add(R(sx, ry + 6, sw_, 24, 'url(#g-shift)', rx=6))
            d.add(R(sx, ry + 6, sw_, 2, '#fff', rx=1, op=.25))
            if warn:
                d.add(R(sx - 1, ry + 5, sw_ + 2, 26, 'none', rx=7, stroke='warn', sw=1.5))
            if sw_ >= 90:
                t0 = f'{(22 + a // 2) % 24:02d}:{"30" if a % 2 else "00"}'
                t1 = f'{(22 + b // 2) % 24:02d}:{"30" if b % 2 else "00"}'
                d.add(T(sx + 10, ry + 22, f'{t0}–{t1}', 12, '#fff', weight=600))
        if not segs:
            d.add(T(GX + 352, ry + 22, 'day off', 12, 'muted', 'middle'))
        hv = hours[i]
        d.add(T(1010, ry + 22, f'{hv:.1f}'.replace('.', ',') if hv else '—', 13, 'text', 'middle', 600))
        d.add(T(1090, ry + 22, f'{int(hv*150):,}'.replace(',', ' ') if hv else '—', 13, 'text2', 'middle'))
        e = s[4]
        d.add(bar(1150, ry + 15, 70, 5, e, need_col(e)))
        if e < .25:
            d.add(icon('warn', 1226, ry + 10, 14, 'bad'))
    # drag ghost (Olga row, extending earlier)
    gy = ROWY + 4 * 40
    gx = GX + 6 * SW + 2
    d.add(R(gx, gy + 6, 2 * SW - 2, 24, 'accent', rx=6, op=.22, stroke='accent_hi', sw=1.5, extra='stroke-dasharray="4 3"'))
    d.add(icon('cursor', gx + 16, gy + 18, 18, 'text'))
    d.add(R(gx - 6, gy - 22, 56, 22, '#0B0A18', rx=6, op=.92))
    d.add(T(gx + 22, gy - 7, '+1 h', 12, 'text', 'middle', 700))
    # coverage
    cy = ROWY + 7 * 40 + 8
    d.add(divider(40, cy - 4, 1200))
    d.add(T(40, cy + 20, 'Coverage', 13, 'text', weight=600))
    d.add(T(120, cy + 20, 'have / need', 11, 'muted'))
    need = [3, 3, 4, 4, 4, 5, 4, 4, 3, 3, 3, 3, 4, 5, 4, 3]
    for j in range(16):
        have = sum(1 for i in range(7) for sg in shifts[i] if not isinstance(sg, str) and sg[0] <= j < sg[1])
        tone = 'good' if have >= need[j] else ('warn' if have == need[j] - 1 else 'bad')
        d.add(R(GX + j * SW + 3, cy + 4, SW - 6, 22, tone, rx=6, op=.18))
        d.add(T(GX + j * SW + SW / 2, cy + 19, f'{have}/{need[j]}', 12, tone, 'middle', 700))
    # guest forecast
    gy0, gh = cy + 36, 56
    d.add(T(40, gy0 + 22, 'Guests', 13, 'text', weight=600))
    d.add(T(40, gy0 + 38, 'hourly forecast', 11, 'muted'))
    vals = [.45, .7, .55, .5, .8, .6, .35, .25, .2, .18, .15, .2, .25, .55, .75, .4, .3]
    pts = [(GX + k * SW, gy0 + gh - v * gh) for k, v in enumerate(vals)]
    path = f'M{pts[0][0]} {gy0+gh}L' + 'L'.join(f'{px:.0f} {py:.0f}' for px, py in pts) + f'L{pts[-1][0]} {gy0+gh}z'
    d.add(f'<path d="{path}" fill="url(#g-guests)"/>')
    d.add('<path d="M' + 'L'.join(f'{px:.0f} {py:.0f}' for px, py in pts) + f'" fill="none" stroke="{C["cyan"]}" stroke-width="2" stroke-linejoin="round"/>')
    for t, lab in ((47 / 60, '22:47'), (2.25, '00:15'), (6 + 52 / 60, '04:52')):
        tx = GX + t * 88
        d.add(R(tx - .5, gy0 + 4, 1, gh - 4, 'amber', op=.6, extra='stroke-dasharray="2 2"'))
        d.add(icon('train', tx + 4, gy0 + 2, 14, 'amber'))
        d.add(T(tx + 22, gy0 + 13, lab, 11, 'amber', weight=600))
    d.add(divider(40, 552, 1200))
    d.add(T(40, 572, 'Drag along a row — shift  ·  Shift — break  ·  right-click — erase  ·  Ctrl+Z — undo', 12, 'text2'))
    d.add(T(1240, 573, 'Total 30.5 h · 4 575 Kč', 14, 'text', 'end', 700))
    # inline warning
    d.note('Inline warning instead of a dialog')
    d.add(R(24, 596, 1232, 36, 'warn', rx=8, op=.12))
    d.add(R(24.5, 596.5, 1231, 35, 'none', rx=8, stroke='warn', extra='stroke-opacity=".35"'))
    d.add(icon('warn', 38, 605, 18, 'warn'))
    d.add(T(64, 619, 'Míša Krejčí: energy 22 % — six hours in a row is risky (mistakes, guest complaints).', 13, 'text'))
    d.add(button(1040, 600, 120, 28, 'Cut to 4 h', 'normal', size=12))
    d.add(button(1166, 600, 82, 28, 'Keep', 'ghost', size=12))
    # bottom actions
    d.add(button(24, 652, 148, 32, 'Undo', 'normal', ic='undo', key='⌃Z'))
    d.add(button(180, 652, 152, 32, 'Like yesterday', 'normal', ic='copy'))
    d.add(button(340, 652, 120, 32, 'Clear', 'ghost', ic='trash'))
    d.add(button(1076, 652, 180, 32, 'Start the night', 'primary', ic='play', key='Enter', icol='#fff'))
    d.save()


def screen_build():
    d = Doc('build', 'Shiftella — Build and furniture')
    cafe_map(d, build=True)
    base_hud(d, roster=False, active='B')
    # ghost placement on counter
    d.note('Placement ghost + price tag by the cursor')
    d.add(R(700, 216, 70, 52, 'good', rx=8, op=.14, stroke='good', sw=2, extra='stroke-dasharray="6 4"'))
    d.add(coffee_machine(705, 222, ghost=True))
    d.add(panel(760, 178, 92, 26, rx=8))
    d.add(T(806, 196, '−3 400 Kč', 12, 'amber', 'middle', 700))
    d.add(icon('cursor', 752, 228, 22, 'text'))
    x, y, w, h = 12, 512, 1000, 148
    d.note('Build panel (B): x12 y512 1000×148, anchor bottom left above the toolbar')
    d.add(panel(x, y, w, h, rx=12))
    tx = 24
    for i, lbl in enumerate(('All', 'Furniture', 'Appliances', 'Counters', 'Decor')):
        bw = tw(lbl, 12, True) + 24
        d.add(button(tx, 524, bw, 26, lbl, 'active' if i == 0 else 'ghost', size=12, rx=13))
        tx += bw + 4
    d.add(button(tx + 8, 524, 112, 26, 'Demolish', 'danger', ic='trash', key='X', size=12))
    d.add(T(1000, 541, 'R — rotate  ·  right-click / Esc — cancel  ·  Tab — tabs', 12, 'muted', 'end'))
    items = [('table', 'Table', '1 200 Kč', 'ok'), ('chair', 'Chair', '450 Kč', 'ok'), ('machine', 'Espresso', '3 400 Kč', 'sel'),
             ('stove', 'Stove', '14 900 Kč', 'no'), ('counter', 'Counter', '900 Kč', 'ok'), ('leaf', 'Plant', '350 Kč', 'ok'),
             ('neon', 'Holo sign', '1 800 Kč', 'ok'), ('lamp', 'Lamp', '600 Kč', 'ok')]
    for i, (ic, name, price, st) in enumerate(items):
        cx, cy = 24 + i * 92, 560
        if st == 'sel':
            d.add(R(cx, cy, 84, 88, 'accent', rx=10, op=.14, stroke='accent', sw=1.5))
        else:
            d.add(R(cx, cy, 84, 88, '#fff', rx=10, op=.05))
        d.add(f'<circle cx="{cx+42}" cy="{cy+28}" r="22" fill="#fff" fill-opacity=".05"/>')
        d.add(icon(ic, cx + 26, cy + 12, 32, 'text' if st != 'no' else 'muted'))
        d.add(T(cx + 42, cy + 64, name, 12, 'text' if st != 'no' else 'muted', 'middle', 500))
        d.add(T(cx + 42, cy + 80, price, 12, 'amber' if st != 'no' else 'bad', 'middle', 700))
    d.add(R(772, 560, 1, 88, '#fff', op=.08))
    ix = 788
    d.add(T(ix, 578, 'Coffee machine «Rapid»', 15, 'text', weight=700, cls='h'))
    d.add(T(ix, 598, '3 400 Kč  ·  goes on a counter', 12, 'amber', weight=600))
    d.add(T(ix, 618, 'Bar: speed +25 %', 12, 'good'))
    d.add(T(ix, 636, 'Noise +1  ·  size 1×1', 12, 'text2'))
    d.save()


def screen_tooltip_event():
    d = Doc('tooltip-and-event', 'Shiftella — guest tooltip and event')
    cafe_map(d, mode='blackout')
    base_hud(d, ticker=dict(text='Power cut — the whole station', t='01:58', tone='bad', ic='bulb_off'))
    # warn marks over two guests
    for gx, gy in ((182, 300), (462, 400)):
        d.add(f'<circle cx="{gx}" cy="{gy}" r="10" fill="#17162E" fill-opacity=".9"/>')
        d.add(icon('warn', gx - 7, gy - 7, 14, 'warn'))
    # guest tooltip
    d.note('Guest tooltip: anchored to the cursor +16/+14, w 264')
    d.add(icon('cursor', 372, 452, 20, 'text'))
    tx, ty, tw_ = 388, 470, 264
    d.add(panel(tx, ty, tw_, 196, rx=10, op=.95))
    d.add(T(tx + 14, ty + 24, 'Student', 14, 'text', weight=700))
    c, cw = chip(tx + tw_ - 14 - (tw('train 00:15', 11) + 40), ty + 10, 'train 00:15', 'neutral', ic='train', h=22, size=11)
    d.add(c)
    d.add(divider(tx + 12, ty + 40, tw_ - 24))
    d.add(icon('bubble', tx + 14, ty + 50, 16, 'cyan'))
    d.add(T(tx + 38, ty + 63, 'cappuccino + croissant', 13, 'text', weight=500))
    d.add(T(tx + 38, ty + 80, 'order being made · ~2 min', 12, 'text2'))
    d.add(icon('hourglass', tx + 14, ty + 92, 16, 'warn'))
    d.add(T(tx + 38, ty + 105, 'Patience', 13, 'text'))
    d.add(bar(tx + 110, ty + 97, 90, 6, .42, 'warn'))
    d.add(T(tx + tw_ - 14, ty + 105, '~6 min', 12, 'text2', 'end', 600))
    d.add(icon('smile', tx + 14, ty + 118, 16, 'good'))
    d.add(T(tx + 38, ty + 131, 'Happy', 13, 'good', weight=600))
    d.add(T(tx + tw_ - 14, ty + 131, '+12', 13, 'good', 'end', 700))
    for i, (sgn, txt, tone) in enumerate((('+8', 'candles — romantic', 'good'), ('+6', 'a warm cappuccino', 'good'), ('−4', 'dark, and a long wait', 'bad'))):
        yy = ty + 152 + i * 17
        d.add(T(tx + 38, yy, sgn, 12, tone, weight=700))
        d.add(T(tx + 64, yy, txt, 12, 'text2'))
    # toasts: collapsed (older) + expanded (newest)
    x = 948
    d.note('Toast stack: anchor bottom right (12, 12), w 320, newest at the bottom')
    ex_h = 196
    ey = 708 - ex_h
    toast(d, x, ey - 8 - 56, 'Train delayed', '00:15 → 00:35 · expect a crowd', 'train', 'info', prog=.35)
    d.add(panel(x, ey, 320, ex_h, rx=10))
    d.add(f'<rect x="{x}" y="{ey+10}" width="3" height="{ex_h-20}" rx="1.5" fill="{C["bad"]}"/>')
    d.add(f'<circle cx="{x+30}" cy="{ey+28}" r="16" fill="{C["bad"]}" fill-opacity=".14"/>')
    d.add(icon('bulb_off', x + 20, ey + 18, 20, 'bad'))
    d.add(T(x + 56, ey + 24, 'Power cut', 13, 'text', weight=700))
    d.add(T(x + 56, ey + 42, 'the whole station · ~20 min', 12, 'text2'))
    d.add(icon('target', x + 262, ey + 12, 16, 'text2'))
    d.add(icon('chev_u', x + 290, ey + 12, 16, 'text2'))
    d.add(divider(x + 14, ey + 56, 292))
    for i, ln in enumerate(('The kitchen and the coffee machine are down.',
                            'Guests get nervous in the dark, and anxious', 'Míša loses mood. But candles are cozy.')):
        d.add(T(x + 14, ey + 76 + i * 18, ln, 12, 'text2'))
    d.add(button(x + 14, ey + 132, 160, 28, 'Light candles', 'primary', ic='candle', size=12, icol='#fff'))
    d.add(T(x + 160, ey + 151, '', 12))
    d.add(button(x + 180, ey + 132, 126, 28, 'Close kitchen', 'normal', size=12))
    d.add(T(x + 14, ey + 182, '−150 Kč  ·  buttons work while paused', 11, 'muted'))
    d.add(T(x + 306, ey + 182, 'F', 11, 'muted', 'end', 700))
    d.save()


def menu_bg(d, rng):
    d.add(R(0, 0, W, H, 'url(#g-sky)'))
    # stars / haze
    for _ in range(40):
        d.add(f'<circle cx="{rng.uniform(0, W):.0f}" cy="{rng.uniform(0, 260):.0f}" r="{rng.uniform(.6, 1.3):.1f}" fill="#fff" opacity="{rng.uniform(.15, .5):.2f}"/>')
    # distant town + church tower
    d.add('<path d="M0 430V360h60v-30h50v40h40v-60h20l14-40 14 40h20v80h70v-24h60v34h80v-50h40v60h90v-30h60v30h80v-40h60v50h120v-20h80v30h200v-40h80v40h52v70z" fill="#1A1838"/>')
    for wx, wy in ((80, 345), (120, 380), (232, 350), (420, 390), (520, 380), (700, 395), (990, 390)):
        d.add(R(wx, wy, 6, 8, 'amber', rx=1, op=.55))
    # train passing
    d.add(R(0, 418, 560, 66, '#2B2A4C', rx=14))
    for i in range(9):
        d.add(R(22 + i * 60, 432, 40, 22, '#818CF8', rx=4, op=.35 + (i % 3) * .1))
    d.add(R(0, 470, 560, 4, 'accent', op=.5))
    d.add(R(0, 484, W, 6, '#3A3A5E'))
    # café facade
    d.add(R(640, 250, 560, 270, 'url(#g-facade)', rx=6))
    d.add(R(640, 250, 560, 10, '#fff', op=.05))
    d.add('<path d="M660 330h520l-20 34H680z" fill="#4F46E5"/>')
    for i in range(13):
        d.add(f'<path d="M{660+i*40} 330h20l-1.5 34h-20z" fill="#2A2748" opacity=".55"/>')
    d.add(R(690, 372, 320, 130, 'url(#g-warmwin)', rx=6))
    d.add(R(690, 372, 320, 130, '#fff', rx=6, op=.06))
    for sx, sy, c in ((760, 470, '#3C6FA8'), (850, 460, '#6A4B8A'), (930, 474, '#2F7D52')):
        d.add(f'<circle cx="{sx}" cy="{sy-22}" r="13" fill="#5A3324" opacity=".75"/><rect x="{sx-22}" y="{sy-8}" width="44" height="40" rx="18" fill="#5A3324" opacity=".75"/>')
    d.add(R(1040, 380, 80, 140, '#3A2A20', rx=4))
    d.add(R(1048, 390, 64, 70, 'url(#g-warmwin)', rx=3, op=.8))
    # neon sign
    d.add('<ellipse cx="910" cy="296" rx="200" ry="60" fill="url(#g-neon)"/>')
    d.add(R(790, 272, 240, 46, '#120E20', rx=8))
    d.add('<text x="910" y="305" font-size="28" font-weight="800" class="h" fill="#A5B4FC" text-anchor="middle" letter-spacing="5" filter="url(#glow)">KAVÁRNA</text>')
    d.add(T(1160, 290, '22–06', 14, 'cyan', 'end', 700))
    # ground + reflections
    d.add(R(0, 520, W, 200, '#11102A'))
    d.add('<path d="M690 520h320l70 200H620z" fill="#FFC46B" opacity=".10" filter="url(#blur8)"/>')
    d.add('<ellipse cx="910" cy="560" rx="170" ry="10" fill="#6366F1" opacity=".30" filter="url(#blur8)"/>')
    d.add('<ellipse cx="850" cy="610" rx="190" ry="9" fill="#FFC46B" opacity=".22" filter="url(#blur8)"/>')
    d.add('<rect x="700" y="540" width="300" height="3" rx="1.5" fill="#FFC77F" opacity=".35"/><rect x="760" y="575" width="220" height="2" rx="1" fill="#A5B4FC" opacity=".35"/>')
    d.add(rain_path(rng, [(0, 0, W, H)], 520))


def screen_main_menu():
    d = Doc('main-menu', 'Shiftella — main menu')
    rng = random.Random(11)
    menu_bg(d, rng)
    d.add('<rect width="640" height="720" fill="url(#g-left)"/>')
    d.extra_defs.append('<linearGradient id="g-left" x1="0" x2="1"><stop offset="0" stop-color="#0B0A18" stop-opacity=".85"/><stop offset="1" stop-color="#0B0A18" stop-opacity="0"/></linearGradient>')
    d.note('Logo — a separate sprite; placeholder text here')
    d.add('<text x="64" y="152" font-size="52" font-weight="800" class="h" fill="#6366F1" opacity=".55" filter="url(#glow)">Shiftella</text>')
    d.add('<text x="64" y="152" font-size="52" font-weight="800" class="h" fill="#EEF0FF">Shiftella</text>')
    d.add(T(66, 182, 'U Výhybky  ·  22:00–06:00', 15, 'text2', weight=500))
    items = [('continue', 'Continue', 'night 3 · 01:40'), ('book', 'Story', ''), ('cube', 'Sandbox', ''),
             ('sliders', 'Settings', ''), ('exit', 'Quit', '')]
    x, y = 56, 250
    d.add(panel(x, y, 300, 16 + len(items) * 44, rx=14, op=.82))
    for i, (ic, lbl, hint) in enumerate(items):
        iy = y + 8 + i * 44
        if i == 2:
            d.add(R(x + 8, iy, 284, 40, 'accent', rx=10, op=.16))
            d.add(R(x + 8, iy + 10, 3, 20, 'accent', rx=1.5))
        d.add(icon(ic, x + 24, iy + 10, 20, 'accent_hi' if i == 2 else 'text2'))
        d.add(T(x + 56, iy + 26, lbl, 16, 'text', weight=600))
        if hint:
            d.add(T(x + 284, iy + 26, hint, 12, 'muted', 'end'))
        if i == 2:
            d.add(icon('chev_r', x + 266, iy + 12, 16, 'accent_hi'))
    d.add(T(64, 690, '↑ ↓ — select  ·  Enter — open  ·  Esc — back', 12, 'text2'))
    d.add(T(1256, 690, 'v0.1.0', 12, 'muted', 'end'))
    d.save()


def screen_sandbox():
    d = Doc('sandbox-setup', 'Shiftella — sandbox setup')
    rng = random.Random(11)
    menu_bg(d, rng)
    d.add(R(0, 0, W, H, '#0B0A18', op=.72))
    x, y, w, h = 240, 40, 800, 640
    d.note('Sandbox screen: centred panel 800×624')
    d.add(panel(x, y, w, h, rx=14, op=.96))
    d.add(T(x + 32, y + 42, 'Sandbox — new game', 18, 'text', weight=700, cls='h'))
    d.add(close_btn(x + w - 40, y + 22))
    d.add(divider(x + 24, y + 62, w - 48))
    LX, CX = x + 32, x + 220

    def label(yy, s, sub=None):
        d.add(T(LX, yy + 21, s, 13, 'text', weight=600))
        if sub:
            d.add(T(LX, yy + 38, sub, 11, 'muted'))

    yy = y + 82
    label(yy, 'Café name')
    d.add(R(CX, yy, 320, 32, 'bg0', rx=6, op=.7, stroke='accent', sw=1.5))
    d.add(T(CX + 12, yy + 21, 'U Výhybky', 13, 'text'))
    d.add(R(CX + 12 + tw('U Výhybky', 13) * .86 + 2, yy + 8, 1.5, 16, 'accent'))
    yy += 52
    label(yy, 'Difficulty')
    for i, lbl in enumerate(('Cozy', 'Classic')):
        d.add(button(CX + i * 136, yy, 132, 32, lbl, 'active' if i == 0 else 'normal'))
    d.add(T(CX, yy + 52, 'No game over, soft payments, patient guests.', 12, 'text2'))
    yy += 76
    label(yy, 'Night character', 'how many events, and which')
    tx0, tx1 = CX, CX + 420
    d.add(R(tx0, yy + 14, 420, 4, '#fff', rx=2, op=.1))
    d.add(R(tx0, yy + 14, 420 * .5, 4, 'url(#g-night)', rx=2))
    for k in range(5):
        d.add(f'<circle cx="{tx0 + k*105}" cy="{yy+16}" r="3" fill="#fff" opacity=".35"/>')
    d.add(f'<circle cx="{tx0 + 210}" cy="{yy+16}" r="9" fill="#fff" filter="url(#sh-s)"/>')
    d.add(T(tx0, yy + 42, 'quiet', 12, 'text2'))
    d.add(T(tx1, yy + 42, 'chaotic', 12, 'text2', 'end'))
    c, _ = chip(tx1 + 24, yy + 5, 'changeable', 'accent', h=24)
    d.add(c)
    d.add(T(CX, yy + 64, '2–4 events a night, now and then a big one: a power cut, an inspection, a wedding.', 12, 'text2'))
    yy += 88
    label(yy, 'Start money')
    for i, lbl in enumerate(('10 000 Kč', '25 000 Kč', '50 000 Kč')):
        d.add(button(CX + i * 164, yy, 160, 32, lbl, 'active' if i == 1 else 'normal'))
    yy += 52
    label(yy, 'Seed')
    d.add(R(CX, yy, 200, 32, 'bg0', rx=6, op=.7, stroke='line'))
    d.add(T(CX + 12, yy + 21, '4F2A-91C3', 14, 'text', weight=600, ls='1'))
    d.add(button(CX + 208, yy, 32, 32, '', 'normal', ic='dice'))
    d.add(button(CX + 248, yy, 132, 32, 'Copy', 'ghost', ic='copy'))
    d.add(T(CX + 392, yy + 21, 'R — random', 12, 'muted'))
    d.add(T(CX, yy + 52, 'Same seed — same guests, events and weather.', 12, 'text2'))
    yy += 72
    label(yy, 'Options')
    for i, (lbl, on) in enumerate((('Rain every night', True), ('Pause on important events', False), ('Allow going into debt', True))):
        oy = yy + 4 + i * 28
        if on:
            d.add(R(CX, oy, 20, 20, 'url(#g-accent)', rx=5))
            d.add(icon('check', CX + 3, oy + 3, 14, '#fff'))
        else:
            d.add(R(CX + .5, oy + .5, 19, 19, 'bg0', rx=5, op=.6, stroke='muted'))
        d.add(T(CX + 32, oy + 15, lbl, 13, 'text'))
    yy += 96
    d.add(R(LX, yy, 736, 48, 'bg0', rx=8, op=.55))
    d.add(icon('info', LX + 14, yy + 14, 20, 'cyan'))
    d.add(T(LX + 46, yy + 22, 'Expect: 2–4 events a night · 30–50 guests · start 25 000 Kč', 13, 'text'))
    d.add(T(LX + 46, yy + 40, 'Best run on this seed: none yet', 12, 'muted'))
    d.add(button(LX, y + h - 52, 132, 32, 'Back', 'normal', key='Esc'))
    d.add(button(x + w - 32 - 168, y + h - 52, 168, 32, 'Start', 'primary', ic='play', key='Enter', icol='#fff'))
    d.save()


def screen_components():
    d = Doc('components', 'Shiftella — components and tokens')
    d.add(R(0, 0, W, H, 'bg1'))
    d.add(T(32, 44, 'Components and tokens', 20, 'text', weight=700, cls='h'))
    d.add(T(32, 66, 'reference screen 1280×720 pt · Inter · icons 24 pt, 2 pt stroke', 12, 'text2'))
    # palette
    d.add(section(32, 100, 'Palette'))
    sw = [('bg0', 'background'), ('panel', 'panel'), ('raised', 'field'), ('hover', 'hover'), ('line', 'line'),
          ('text', 'text'), ('text2', 'text 2'), ('muted', 'muted'), ('accent', 'accent'), ('accent_hi', 'accent hi'),
          ('accent_lo', 'accent lo'), ('cyan', 'info'), ('violet', 'night'), ('amber', 'money/light'), ('honey', 'honey'),
          ('good', 'good'), ('warn', 'warning'), ('bad', 'bad')]
    for i, (k, lbl) in enumerate(sw):
        cx, cy = 32 + (i % 9) * 68, 112 + (i // 9) * 78
        d.add(R(cx, cy, 60, 40, k, rx=8, stroke='#fff', extra='stroke-opacity=".08"'))
        d.add(T(cx, cy + 54, lbl, 11, 'text'))
        d.add(T(cx, cy + 68, C[k], 10, 'muted'))
    # type
    d.add(section(680, 100, 'Fonts'))
    for i, (s, lbl, wt, cls) in enumerate(((18, 'Title 18 · Inter 700', 700, 'h'), (16, 'Heading 16 · 700', 700, 'h'),
                                            (13, 'Body 13 · Inter 400 — Příliš žluťoučký kůň', 400, None),
                                            (13, 'Strong 13 · Inter 600 — 12 480 Kč', 600, None),
                                            (12, 'Small 12 · Inter 400 — captions, secondary', 400, None),
                                            (11, 'SECTION 11 · INTER 700 · TRACKING 0.6', 700, None))):
        d.add(T(680, 128 + i * 26, lbl, s, 'text' if i < 4 else 'text2', weight=wt, cls=cls))
    # buttons
    d.add(section(32, 300, 'Buttons · 28 pt (standard), 24 (compact), 32 (screen)'))
    bx = 32
    for st in ('normal', 'hover', 'pressed', 'active', 'primary', 'disabled', 'ghost'):
        lbl = {'normal': 'Normal', 'hover': 'Hover', 'pressed': 'Pressed', 'active': 'Selected', 'primary': 'Primary',
               'disabled': 'Disabled', 'ghost': 'Ghost', 'danger': 'Danger'}[st]
        bw = tw(lbl, 13, True) + 28
        d.add(button(bx, 312, bw, 28, lbl, st))
        bx += bw + 8
    d.add(button(32, 352, 132, 28, 'Hire', 'normal', ic='plus', key='H'))
    d.add(button(532, 352, 100, 28, 'Demolish', 'danger', ic='trash', size=12))
    d.add(R(170, 350, 136, 32, 'none', rx=8, stroke='cyan', sw=2))
    d.add(button(174, 352, 128, 28, 'Focus (Tab)', 'normal'))
    for i, ic in enumerate(('pause', 'play', 'ff2', 'ff3')):
        d.add(button(316 + i * 30, 352, 28, 28, '', 'active' if i == 1 else 'normal', ic=ic))
    kc, _ = keycap(448, 357, 'Space')
    d.add(kc)
    kc, _ = keycap(500, 357, 'C')
    d.add(kc)
    # controls
    d.add(section(32, 412, 'Need bars · chips · fields'))
    for i, v in enumerate((.82, .42, .18)):
        d.add(bar(32, 428 + i * 14, 120, 6, v, need_col(v)))
    cx = 172
    for lbl, tone in (('Night Owl', 'good'), ('Chatterbox', 'neutral'), ('Anxious', 'bad'), ('changeable', 'accent'), ('rain 80 %', 'info')):
        c, cw = chip(cx, 426, lbl, tone)
        d.add(c)
        cx += cw + 6
    d.add(R(172, 458, 200, 32, 'bg0', rx=6, op=.7, stroke='line'))
    d.add(T(184, 479, 'Input field', 13, 'text2'))
    d.add(R(380, 458, 200, 32, 'bg0', rx=6, op=.7, stroke='accent', sw=1.5))
    d.add(T(392, 479, 'Focus', 13, 'text'))
    d.add(R(592, 463, 20, 20, 'url(#g-accent)', rx=5))
    d.add(icon('check', 595, 466, 14, '#fff'))
    d.add(R(620.5, 463.5, 19, 19, 'bg0', rx=5, op=.6, stroke='muted'))
    # priority cells
    for i, v in enumerate((1, 2, 3, 4, 0)):
        fills = {1: ('url(#g-accent)', None, '#fff'), 2: ('amber', .9, '#2A1A0A'), 3: ('#fff', .14, 'text'), 4: ('#fff', .07, 'text2'), 0: ('none', None, 'muted')}
        f, o, tc = fills[v]
        px = 172 + i * 56
        if f != 'none':
            d.add(R(px, 500, 48, 26, f, rx=6, op=o))
        else:
            d.add(R(px + .5, 500.5, 47, 25, 'none', rx=6, stroke='#fff', extra='stroke-opacity=".1" stroke-dasharray="3 3"'))
        d.add(T(px + 24, 518, str(v) if v else '—', 14, tc, 'middle', 700))
    d.add(R(452, 500, 48, 26, 'url(#p-hatch)', rx=6))
    d.add(T(476, 518, '✕', 12, 'bad', 'middle', 700))
    d.add(T(32, 518, 'Priority cells', 12, 'text2'))
    # panel + toast
    d.add(section(32, 552, 'Panel · toast'))
    d.add(panel(32, 564, 260, 144, rx=12))
    d.add(T(48, 590, 'Panel', 17, 'text', weight=700, cls='h'))
    d.add(close_btn(256, 574))
    d.add(divider(44, 604, 236))
    d.add(T(48, 626, 'radius 12 · fill 92 %', 12, 'text2'))
    d.add(T(48, 644, 'shadow 0/6/16 · #000 45 %', 12, 'text2'))
    d.add(T(48, 662, 'edge 1 pt · white 7 %', 12, 'text2'))
    d.add(T(48, 680, 'padding 12–16 pt', 12, 'text2'))
    toast(d, 316, 564, 'Power cut', 'the whole station · ~20 min', 'bulb_off', 'bad', prog=.6)
    toast(d, 316, 628, 'Train delayed', '00:15 → 00:35 · expect a crowd', 'train', 'info', prog=.3)
    # icons
    d.add(section(680, 300, 'Icons 24 pt · 2 pt stroke · round caps'))
    names = [k for k in ICONS.keys() if k not in ('cursor',)]
    for i, k in enumerate(names):
        cx, cy = 680 + (i % 16) * 36, 312 + (i // 16) * 36
        d.add(R(cx, cy, 34, 34, '#fff', rx=8, op=.04))
        d.add(icon(k, cx + 5, cy + 5, 24, 'text'))
    # world character + furniture sample
    d.add(section(680, 476, 'World (v2 top-down): character, furniture, light'))
    d.add(R(680, 488, 568, 200, 'url(#p-planks)', rx=12))
    d.add(f'<circle cx="820" cy="588" r="110" fill="url(#g-lamp)" style="mix-blend-mode:screen" opacity=".6"/>')
    d.add(chair(786, 588, -90))
    d.add(chair(854, 588, 90))
    d.add(round_table(820, 588))
    d.add(person(748, 588, '#3E7C8C', '#2A1A14', 'skin1', rot=-90))
    d.add(person(960, 590, '#4A6FB8', '#7A3B2A', 'skin3', apron='#6366F1', rot=0, sel=True))
    d.add(person(1040, 590, '#2F7D52', '#1C1410', 'skin2', apron='#6366F1', rot=90))
    d.add(coffee_machine(1110, 566))
    d.add(plant(1210, 640, 16))
    d.save()


if __name__ == '__main__':
    screen_hud()
    screen_staff()
    screen_priorities()
    screen_schedule()
    screen_build()
    screen_tooltip_event()
    screen_main_menu()
    screen_sandbox()
    screen_components()
