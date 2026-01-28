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
- Background colors (except white)
- Icons or images
- Complex navigation
- Hover effects beyond underlines
- Any decoration that doesn't serve readability
