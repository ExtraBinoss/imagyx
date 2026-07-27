export type FloatingSide = 'top' | 'right' | 'bottom' | 'left'
export type FloatingAlign = 'start' | 'center' | 'end'

export interface FloatingPositionOptions {
  side: FloatingSide
  align?: FloatingAlign
  gap?: number
  viewportPadding?: number
}

export interface FloatingPosition {
  left: number
  top: number
  side: FloatingSide
  maxWidth: number
  maxHeight: number
}

const oppositeSide: Record<FloatingSide, FloatingSide> = {
  top: 'bottom',
  right: 'left',
  bottom: 'top',
  left: 'right',
}

function availableSpace(anchor: DOMRect, side: FloatingSide, viewportWidth: number, viewportHeight: number, padding: number, gap: number) {
  if (side === 'top') return anchor.top - padding - gap
  if (side === 'bottom') return viewportHeight - anchor.bottom - padding - gap
  if (side === 'left') return anchor.left - padding - gap
  return viewportWidth - anchor.right - padding - gap
}

function fits(content: DOMRect, side: FloatingSide, space: number) {
  return side === 'top' || side === 'bottom' ? content.height <= space : content.width <= space
}

export function computeFloatingPosition(
  anchor: DOMRect,
  content: DOMRect,
  options: FloatingPositionOptions,
): FloatingPosition {
  const gap = options.gap ?? 8
  const padding = options.viewportPadding ?? 8
  const align = options.align ?? 'center'
  const viewportWidth = document.documentElement.clientWidth
  const viewportHeight = document.documentElement.clientHeight
  const preferred = options.side
  const opposite = oppositeSide[preferred]
  const preferredSpace = availableSpace(anchor, preferred, viewportWidth, viewportHeight, padding, gap)
  const oppositeSpace = availableSpace(anchor, opposite, viewportWidth, viewportHeight, padding, gap)
  const side = !fits(content, preferred, preferredSpace) && oppositeSpace > preferredSpace ? opposite : preferred

  let left = anchor.left
  let top = anchor.bottom + gap

  if (side === 'top') top = anchor.top - content.height - gap
  if (side === 'left') left = anchor.left - content.width - gap
  if (side === 'right') left = anchor.right + gap

  if (side === 'top' || side === 'bottom') {
    if (align === 'center') left = anchor.left + (anchor.width - content.width) / 2
    if (align === 'end') left = anchor.right - content.width
  } else {
    if (align === 'start') top = anchor.top
    if (align === 'center') top = anchor.top + (anchor.height - content.height) / 2
    if (align === 'end') top = anchor.bottom - content.height
  }

  const maxLeft = Math.max(padding, viewportWidth - content.width - padding)
  const maxTop = Math.max(padding, viewportHeight - content.height - padding)

  return {
    left: Math.min(Math.max(left, padding), maxLeft),
    top: Math.min(Math.max(top, padding), maxTop),
    side,
    maxWidth: Math.max(0, viewportWidth - padding * 2),
    maxHeight: Math.max(0, viewportHeight - padding * 2),
  }
}
