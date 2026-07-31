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

const COLOR_FRENCH: Record<CanonicalColor, string> = {
  red: 'rouge',
  green: 'vert',
  blue: 'bleu',
  yellow: 'jaune',
  orange: 'orange',
  purple: 'violet',
  pink: 'rose',
  black: 'noir',
  white: 'blanc',
  gray: 'gris',
  brown: 'marron',
  cyan: 'cyan',
  beige: 'beige',
}

const ALIAS_TO_COLOR = new Map<string, CanonicalColor>()
for (const [color, aliases] of Object.entries(COLOR_ALIASES) as [CanonicalColor, readonly string[]][]) {
  for (const alias of aliases) ALIAS_TO_COLOR.set(normalizeWord(alias), color)
}

const QUERY_STOP_WORDS = new Set([
  'a', 'an', 'and', 'avec', 'de', 'des', 'du', 'en', 'et', 'la', 'le', 'les',
  'of', 'par', 'pour', 'the', 'un', 'une', 'with',
])

const SUBJECT_TRANSLATIONS: Record<string, { english: string; french: string; aliases?: string[] }> = {
  woman: { english: 'woman', french: 'femme', aliases: ['girl'] },
  women: { english: 'women', french: 'femmes', aliases: ['girls'] },
  femme: { english: 'woman', french: 'femme', aliases: ['girl'] },
  femmes: { english: 'women', french: 'femmes', aliases: ['girls'] },
  girl: { english: 'girl', french: 'fille', aliases: ['woman'] },
  girls: { english: 'girls', french: 'filles', aliases: ['women'] },
  fille: { english: 'girl', french: 'fille', aliases: ['woman'] },
  filles: { english: 'girls', french: 'filles', aliases: ['women'] },
  man: { english: 'man', french: 'homme', aliases: ['boy'] },
  men: { english: 'men', french: 'hommes', aliases: ['boys'] },
  homme: { english: 'man', french: 'homme', aliases: ['boy'] },
  hommes: { english: 'men', french: 'hommes', aliases: ['boys'] },
  person: { english: 'person', french: 'personne' },
  people: { english: 'people', french: 'personnes' },
  personne: { english: 'person', french: 'personne' },
  personnes: { english: 'people', french: 'personnes' },
  female: { english: 'woman', french: 'femme', aliases: ['girl'] },
  male: { english: 'man', french: 'homme', aliases: ['boy'] },
  human: { english: 'person', french: 'personne' },
  humain: { english: 'person', french: 'personne' },
}

const GRAPHIC_NEGATIVE_PROMPTS = [
  'a logo or brand mark',
  'graphic design with text',
  'an abstract graphic without a person',
  'un logo ou une marque',
  'un graphisme avec du texte',
  'un graphisme abstrait sans personne',
]

type PromptGroup = {
  prompts: string[]
  weight: number
}

export interface QueryPromptPlan {
  positivePrompts: string[]
  positiveGroups: PromptGroup[]
  negativePrompts: string[]
  conceptLabels: string[]
  subjectLabels: string[]
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
  const subjectLabels = conceptLabels.filter((label) => !ALIAS_TO_COLOR.has(normalizeWord(label)))
  const dominantColor = isDominanceQuery(trimmed)

  if (!detected) {
    const subjectPrompts = buildSubjectPrompts(trimmed, subjectLabels)
    return {
      positivePrompts: subjectPrompts,
      positiveGroups: [{ prompts: subjectPrompts, weight: 1 }],
      negativePrompts: [],
      conceptLabels,
      subjectLabels,
      detectedColors: [],
      dominantColor,
      negativeWeight: 0,
    }
  }

  const canonicalQuery = replaceColors(trimmed, conceptLabels, detectedColors)
  const colorOnly = subjectLabels.length === 0
  if (colorOnly) {
    const colorPrompts = unique(detectedColors.flatMap((color) => [
      color,
      `the color ${color}`,
      `a ${color} image`,
      `une image ${COLOR_FRENCH[color]}`,
      `an image dominated by ${color}`,
      `a photo with mostly ${color} colors`,
    ]))
    return {
      positivePrompts: colorPrompts,
      positiveGroups: [{ prompts: colorPrompts, weight: 1 }],
      negativePrompts: buildColorContrastPrompts(detectedColors, canonicalQuery, true, detected),
      conceptLabels,
      subjectLabels,
      detectedColors,
      detectedColor: detected,
      dominantColor,
      negativeWeight: detectedColors.length === 1 ? 0.35 : 0,
    }
  }

  const subjectPrompts = buildSubjectPrompts(subjectLabels.join(' '), subjectLabels)
  const englishSubject = translateSubjectPhrase(subjectLabels, 'english')
  const frenchSubject = translateSubjectPhrase(subjectLabels, 'french')
  const combinedPrompts = unique([
    canonicalQuery,
    `a photo of ${canonicalQuery}`,
    `une photo de ${canonicalQuery}`,
    `an image showing ${canonicalQuery}`,
    `une image montrant ${canonicalQuery}`,
    ...detectedColors.flatMap((color) => {
      const frenchColor = COLOR_FRENCH[color]
      return [
        `a photo of a ${englishSubject} with ${color} colors`,
        `a ${englishSubject} in ${color}`,
        `une photo d'une ${frenchSubject} avec du ${frenchColor}`,
      ]
    }),
  ])

  // The subject receives more weight than the colour. For human queries,
  // contrast prompts about other colours would erase the requested person;
  // graphic distractors are safer for branding-heavy images such as Daweasy.
  const humanSubject = isHumanSubject(subjectLabels)
  const negativePrompts = humanSubject
    ? GRAPHIC_NEGATIVE_PROMPTS
    : buildColorContrastPrompts(detectedColors, canonicalQuery, false, detected)

  return {
    positivePrompts: unique([...subjectPrompts, ...combinedPrompts]),
    positiveGroups: [
      { prompts: subjectPrompts, weight: 3 },
      { prompts: combinedPrompts, weight: 1 },
    ],
    negativePrompts: unique(negativePrompts),
    conceptLabels,
    subjectLabels,
    detectedColors,
    detectedColor: detected,
    dominantColor,
    negativeWeight: humanSubject ? 0.12 : detectedColors.length === 1 ? 0.35 : 0,
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

export function weightedPromptVectors(
  groups: Array<{ vectors: number[][]; weight: number }>,
): number[][] {
  return groups.flatMap(({ vectors, weight }) => {
    const copies = Math.max(1, Math.round(weight))
    return Array.from({ length: copies }, () => vectors).flat()
  })
}

function buildSubjectPrompts(query: string, subjectLabels: string[]): string[] {
  const trimmed = query.trim()
  const english = translateSubjectPhrase(subjectLabels, 'english') || trimmed
  const french = translateSubjectPhrase(subjectLabels, 'french') || trimmed
  const human = isHumanSubject(subjectLabels)
  const englishSubject = human ? `a ${english}` : english
  const aliases = subjectLabels.flatMap((label) => SUBJECT_TRANSLATIONS[normalizeWord(label)]?.aliases ?? [])
  return unique([
    trimmed,
    english,
    `a photo of ${englishSubject}`,
    `une photo de ${french}`,
    `an image showing ${englishSubject}`,
    `une image montrant ${french}`,
    ...aliases.flatMap((alias) => [
      `a photo of a ${alias}`,
      `une photo d'une ${alias}`,
    ]),
  ])
}

function buildColorContrastPrompts(
  colors: CanonicalColor[],
  query: string,
  colorOnly: boolean,
  detected?: CanonicalColor,
): string[] {
  if (colors.length !== 1 || !detected) return []
  return COLOR_CONTRASTS[detected].map((contrast) =>
    colorOnly
      ? `an image dominated by ${contrast}`
      : `a photo of ${replaceFirstColor(query, detected, contrast)}`,
  )
}

function translateSubjectPhrase(
  labels: string[],
  language: 'english' | 'french',
): string {
  return labels
    .map((label) => SUBJECT_TRANSLATIONS[normalizeWord(label)]?.[language] ?? label)
    .join(' ')
    .trim()
}

function isHumanSubject(labels: string[]): boolean {
  return labels.some((label) => Boolean(SUBJECT_TRANSLATIONS[normalizeWord(label)]))
}

function extractConceptLabels(query: string): string[] {
  const seen = new Set<string>()
  return query
    .toLocaleLowerCase('fr')
    .split(/[^\p{L}\p{N}-]+/u)
    .map((word) => word.trim())
    .filter((word) => word.length >= 2)
    .filter((word) => !QUERY_STOP_WORDS.has(normalizeWord(word)))
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
