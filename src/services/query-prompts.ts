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
  detectedColor?: CanonicalColor
  detectedColors: CanonicalColor[]
  dominantColor: boolean
  negativeWeight: number
}

export type QueryColor = CanonicalColor

export interface QueryColorIntent {
  colors: QueryColor[]
  dominant: boolean
}

export function buildQueryPromptPlan(query: string): QueryPromptPlan {
  const trimmed = query.trim()
  const conceptLabels = extractConceptLabels(trimmed)
  const detectedColors = detectColors(conceptLabels)
  const detected = detectedColors[0]
  const dominantColor = isDominanceQuery(trimmed)

  if (!detected) {
    return {
      positivePrompts: unique([
        trimmed,
        `a photo of ${trimmed}`,
        `une photo de ${trimmed}`,
        `an image showing ${trimmed}`,
        `une image montrant ${trimmed}`,
      ]),
      negativePrompts: [],
      conceptLabels,
      detectedColors: [],
      dominantColor,
      negativeWeight: 0,
    }
  }

  const canonicalQuery = replaceColors(trimmed, conceptLabels, detectedColors)
  const colorOnly = conceptLabels.length === detectedColors.length
  const positivePrompts = colorOnly
    ? detectedColors.flatMap((color) => [
        color,
        `the color ${color}`,
        `a ${color} image`,
        `une image ${color}`,
        `an image dominated by ${color}`,
        `a photo with mostly ${color} colors`,
      ])
    : [
        canonicalQuery,
        `a photo of ${canonicalQuery}`,
        `une photo de ${canonicalQuery}`,
        `an image showing ${canonicalQuery}`,
        `une image montrant ${canonicalQuery}`,
        ...detectedColors.map((color) => `an image where ${color} is visually prominent`),
      ]

  // A multi-colour query describes a positive combination. Subtracting a
  // contrast such as "red -> green" would erase one of the requested colors.
  const negativePrompts = detectedColors.length === 1
    ? COLOR_CONTRASTS[detected].map((contrast) =>
    colorOnly
      ? `an image dominated by ${contrast}`
      : `a photo of ${replaceFirstColor(canonicalQuery, detected, contrast)}`,
      )
    : []

  return {
    positivePrompts: unique(positivePrompts),
    negativePrompts: unique(negativePrompts),
    conceptLabels,
    detectedColors,
    dominantColor,
    detectedColor: detected,
    negativeWeight: detectedColors.length === 1 ? 0.35 : 0,
  }
}

export function extractQueryColorIntent(query: string): QueryColorIntent {
  const labels = extractConceptLabels(query.trim())
  return {
    colors: detectColors(labels),
    dominant: isDominanceQuery(query),
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

function detectColors(labels: string[]): CanonicalColor[] {
  return [...new Set(labels.flatMap((label) => {
    const color = ALIAS_TO_COLOR.get(normalizeWord(label))
    return color ? [color] : []
  }))]
}

function replaceFirstColor(query: string, from: string, to: string): string {
  const escaped = from.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  return query.replace(new RegExp(`\\b${escaped}\\b`, 'iu'), to)
}

function replaceColors(query: string, labels: string[], colors: CanonicalColor[]): string {
  return colors.reduce((result, color) => {
    const alias = labels.find((label) => ALIAS_TO_COLOR.get(normalizeWord(label)) === color)
    return alias ? replaceFirstColor(result, alias, color) : result
  }, query)
}

function isDominanceQuery(query: string): boolean {
  return /\b(?:dominant(?:e)?|mostly|principal(?:e|ement)?|predominant(?:e|ly)?)\b/iu.test(query)
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
