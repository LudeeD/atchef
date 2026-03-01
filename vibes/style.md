# AtChef Style Guide

## Philosophy

Ultra-minimal, text-focused design. Let the content breathe.

## Core Principles

- **Single-column layout**, max 600px, centered
- **Text-only** - no cards, boxes, or containers
- **Clean typography** with generous spacing
- **White background**, dark text (#222)
- **Sage green (#5a7d5a)** for links/accents only
- **No visual flourishes** - no shadows, borders, gradients

## Typography

```
Font:        system-ui, sans-serif
Base size:   16px
Line height: 1.6
Text color:  #222
Meta color:  #666
```

## Sizing

| Element | Size |
|---------|------|
| Logo | 18px, weight 600 |
| Page title (h1) | 24px, weight 600 |
| Section heading (h2) | 16px, weight 600 |
| Recipe title in list | 17px |
| Meta text | 14px |
| Comment meta | 13px |

## Spacing

```
Page padding:     20px
Header margin:    30px bottom
Recipe items:     20px bottom
Section headings: 25px top, 10px bottom
Comments section: 40px top, 1px #eee border
Comment indent:   25px left
List items:       8px bottom
```

## Links

- Color: #5a7d5a (sage green)
- No underline by default
- Underline on hover
- Logo is #222, no underline ever

## Layout Patterns

### Header
```
AtChef                              [new]
```
- Logo left, action link right
- Flex with space-between

### Recipe List Item
```
Recipe Title (link)
by author · time ago · N comments
```

### Recipe Page
```
Title (h1)
by author · time ago

Description paragraph

Prep: X · Cook: Y · Serves: Z

Ingredients (h2)
• bullet list

Steps (h2)
1. numbered list

Comments (N) (h2, with top border)
  author · time ago
  comment text
    nested replies indented
```

## What to Avoid

- Cards or box containers
- Background colors (except white/dark)
- Icons or images
- Complex navigation
- Hover effects beyond underlines
- Any decoration that doesn't serve readability

## Themes & Accessibility

### Contrast Standards
- **WCAG AAA compliance** (7:1+ contrast ratio)
- **Enhanced readability** while maintaining minimal aesthetic
- **System preference support** with manual override capability

### Light Mode (Enhanced Contrast)
```
Primary text:    #0a0a0a (was #222) - 15.8:1 contrast ratio
Secondary text:  #2d2d2d (was #666) - 8.3:1 contrast ratio  
Meta text:       #404040 (was #999) - 7.1:1 contrast ratio
Background:      #ffffff
Accent (sage):   #5a7d5a
Error text:      #8b0000 - 7.2:1 contrast ratio
```

### Dark Mode
```
Primary text:    #f8f8f8 on #0a0a0a - 15.5:1 contrast ratio
Secondary text:  #d0d0d0 on #0a0a0a - 8.5:1 contrast ratio
Meta text:       #b0b0b0 on #0a0a0a - 6.8:1 contrast ratio
Background:      #0a0a0a (deepest black)
Surface:         #151515 (subtle elevation)
Accent:          #7bb37b (lighter sage for dark backgrounds)
Error text:      #ff6b6b - 4.8:1 contrast ratio
```

### Theme Behavior
- **Auto-detection**: Respects `prefers-color-scheme` system preference by default
- **Manual override**: User can select Light/Dark/Auto in profile page settings
- **localStorage persistence**: Theme choice saved in browser (no database storage)
- **Minimal JavaScript**: ~50 lines total for theme detection and switching
- **Progressive enhancement**: Falls back gracefully if JavaScript disabled
- **Footer notice**: Subtle notification when dark theme is auto-detected

### Cooklang Syntax Highlighting
**Light Mode:**
- Ingredients: #228833 (enhanced from #2a6)
- Equipment: #996677 (enhanced from #a67) 
- Timers: #4477aa on #e8f0ff background

**Dark Mode:**
- Ingredients: #55cc66 (bright green)
- Equipment: #cc88aa (bright purple)
- Timers: #66aadd on #1a2a3a background

The theme system maintains the ultra-minimal aesthetic while providing excellent accessibility and user choice.
