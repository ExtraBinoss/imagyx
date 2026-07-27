const COLOR_ALIASES = {
  red: ['red', 'rouge'],
  green: ['green', 'vert', 'verte'],
  blue: ['blue', 'bleu', 'bleue'],
  yellow: ['yellow', 'jaune'],
  orange: ['orange'],
  purple: ['purple', 'violet', 'violette'],
  pink: ['pink', 'rose'],
  black: ['black', 'noir', 'noire'],
  white: ['white', 'blanc', 'blanche'],
  gray: ['gray', 'grey', 'gris', 'grise'],
  brown: ['brown', 'marron', 'brun', 'brune'],
  cyan: ['cyan', 'turquoise'],
  beige: ['beige'],
} as const

type CanonicalColor = keyof typeof COLOR_ALIASES

const COLOR_CONTRASTS: Record<CanonicalColor, CanonicalColor[]> = {
  red: ['green', 'blue', 'cyan'],
  green: ['red', 'purple', 'pink'],
  blue: ['red', 'orange', 'yellow'],
  yellow: ['blue', 'purple', 'black'],
  orange: ['blue', 'green', 'purple'],
  purple: ['green', 'yellow', 'orange'],
  pink: ['green', 'blue', 'black'],
  black: ['white', 'beige', 'yellow'],
  white: ['black', 'brown', 'purple'],
  gray: ['red', 'green', 'blue'],
  brown: ['blue', 'cyan', 'white'],
  cyan: ['red', 'orange', 'brown'],
  beige: ['black', 'blue', 'red'],
}

const ALIAS_TO_COLOR = new Map<string, CanonicalColor>()
for (const [color, aliases] of Object.entries(COLOR_ALIASES) as [CanonicalColor, readonly string[]][]) {
  for (const alias of aliases) ALIAS_TO_COLOR.set(normalizeWord(alias), color)
}

export interface QueryPromptPlan {
  positivePrompts: string[]
  negativePrompts: string[]
  conceptLabels: string[]
  conceptPrompts: string[]
  detectedColor?: CanonicalColor
  negativeWeight: number
}

export function buildQueryPromptPlan(query: string): QueryPromptPlan {
  const trimmed = query.trim()
  const conceptLabels = extractConceptLabels(trimmed)
  const detected = detectColor(conceptLabels)

  if (!detected) {
    return {
      positivePrompts: unique([
        trimmed,
        `a photo of ${trimmed}`,
        `an image showing ${trimmed}`,
      ]),
      negativePrompts: [],
      conceptLabels,
      conceptPrompts: conceptLabels.map((label) => `a photo of ${label}`),
      negativeWeight: 0,
    }
  }

  const canonicalQuery = replaceFirstColor(trimmed, detected.alias, detected.color)
  const colorOnly = conceptLabels.length === 1
  const positivePrompts = colorOnly
    ? [
        detected.color,
        `the color ${detected.color}`,
        `a ${detected.color} image`,
        `an image dominated by ${detected.color}`,
        `a photo with mostly ${detected.color} colors`,
      ]
    : [
        canonicalQuery,
        `a photo of ${canonicalQuery}`,
        `an image showing ${canonicalQuery}`,
        `an image where ${detected.color} is visually prominent`,
      ]

  const negativePrompts = COLOR_CONTRASTS[detected.color].map((contrast) =>
    colorOnly
      ? `an image dominated by ${contrast}`
      : `a photo of ${replaceFirstColor(canonicalQuery, detected.color, contrast)}`,
  )

  return {
    positivePrompts: unique(positivePrompts),
    negativePrompts: unique(negativePrompts),
    conceptLabels,
    conceptPrompts: conceptLabels.map((label) => {
      const color = ALIAS_TO_COLOR.get(normalizeWord(label))
      return color ? `an image dominated by ${color}` : `a photo of ${label}`
    }),
    detectedColor: detected.color,
    negativeWeight: 0.35,
  }
}

export function combinePromptVectors(
  positiveVectors: number[][],
  negativeVectors: number[][],
  negativeWeight: number,
): number[] {
  const positive = meanVector(positiveVectors)
  if (!positive.length) return []
  const negative = meanVector(negativeVectors)
  const combined = positive.map((value, index) =>
    value - (negative[index] ?? 0) * negativeWeight,
  )
  return normalizeVector(combined)
}

function extractConceptLabels(query: string): string[] {
  const seen = new Set<string>()
  return query
    .toLocaleLowerCase('fr')
    .split(/[^\p{L}\p{N}-]+/u)
    .map((word) => word.trim())
    .filter((word) => word.length >= 2)
    .filter((word) => {
      const key = normalizeWord(word)
      if (seen.has(key)) return false
      seen.add(key)
      return true
    })
    .slice(0, 6)
}

function detectColor(labels: string[]): { color: CanonicalColor; alias: string } | undefined {
  for (const label of labels) {
    const color = ALIAS_TO_COLOR.get(normalizeWord(label))
    if (color) return { color, alias: label }
  }
  return undefined
}

function replaceFirstColor(query: string, from: string, to: string): string {
  const escaped = from.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  return query.replace(new RegExp(`\\b${escaped}\\b`, 'iu'), to)
}

function meanVector(vectors: number[][]): number[] {
  const dimensions = vectors.find((vector) => vector.length > 0)?.length ?? 0
  if (!dimensions) return []
  const valid = vectors.filter((vector) => vector.length === dimensions)
  if (!valid.length) return []
  const mean = new Array<number>(dimensions).fill(0)
  for (const vector of valid) {
    for (let index = 0; index < dimensions; index += 1) {
      mean[index] += vector[index] ?? 0
    }
  }
  return mean.map((value) => value / valid.length)
}

function normalizeVector(vector: number[]): number[] {
  const norm = Math.sqrt(vector.reduce((sum, value) => sum + value * value, 0))
  if (norm <= Number.EPSILON) return vector.map(() => 0)
  return vector.map((value) => value / norm)
}

function normalizeWord(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLocaleLowerCase('fr')
}

function unique(values: string[]): string[] {
  return [...new Set(values.map((value) => value.trim()).filter(Boolean))]
}
