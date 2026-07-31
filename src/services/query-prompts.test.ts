import { buildQueryPromptPlan, combinePromptVectors } from './query-prompts'

describe('buildQueryPromptPlan', () => {
  it('builds a contrastive plan for an English color', () => {
    const plan = buildQueryPromptPlan('red')

    expect(plan.detectedColor).toBe('red')
    expect(plan.positivePrompts).toContain('an image dominated by red')
    expect(plan.negativePrompts).toContain('an image dominated by green')
    expect(plan.negativeWeight).toBeGreaterThan(0)
  })

  it('normalizes French color aliases to the English CLIP concept', () => {
    const plan = buildQueryPromptPlan('rouge')

    expect(plan.detectedColor).toBe('red')
    expect(plan.positivePrompts).toContain('a red image')
  })

  it('keeps the searched subject while contrasting its color', () => {
    const plan = buildQueryPromptPlan('red car')

    expect(plan.positivePrompts).toContain('a photo of red car')
    expect(plan.negativePrompts).toContain('a photo of green car')
    expect(plan.negativePrompts).toContain('a photo of blue car')
  })

  it('uses a visual prompt ensemble for ordinary queries', () => {
    const plan = buildQueryPromptPlan('mountain lake')

    expect(plan.detectedColor).toBeUndefined()
    expect(plan.positivePrompts).toEqual([
      'mountain lake',
      'a photo of mountain lake',
      'une photo de mountain lake',
      'an image showing mountain lake',
      'une image montrant mountain lake',
    ])
    expect(plan.negativePrompts).toEqual([])
  })

  it('keeps every explicitly requested colour positive', () => {
    const plan = buildQueryPromptPlan('red and green')

    expect(plan.detectedColors).toEqual(['red', 'green'])
    expect(plan.negativePrompts).toEqual([])
    expect(plan.negativeWeight).toBe(0)
  })

  it('keeps a human subject ahead of a requested colour', () => {
    const plan = buildQueryPromptPlan('woman green')

    expect(plan.subjectLabels).toEqual(['woman'])
    expect(plan.detectedColors).toEqual(['green'])
    expect(plan.positivePrompts).toContain('a photo of a woman')
    expect(plan.positivePrompts).toContain('a woman in green')
    expect(plan.positiveGroups[0]?.weight).toBeGreaterThan(plan.positiveGroups[1]?.weight ?? 0)
    expect(plan.negativePrompts).not.toContain('a photo of a red woman')
    expect(plan.negativePrompts).toContain('a logo or brand mark')
  })
})

describe('combinePromptVectors', () => {
  it('moves the query vector away from contrasting concepts', () => {
    const vector = combinePromptVectors([[1, 0]], [[0, 1]], 0.35)

    expect(vector[0]).toBeGreaterThan(0.9)
    expect(vector[1]).toBeLessThan(0)
    expect(Math.hypot(...vector)).toBeCloseTo(1)
  })
})
