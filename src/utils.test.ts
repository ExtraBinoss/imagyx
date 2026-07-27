import { describe, expect, it } from 'vitest'
import { basename, formatBytes } from './utils'

describe('formatBytes', () => {
  it('formats bytes with readable units', () => {
    expect(formatBytes(900)).toBe('900 o')
    expect(formatBytes(2048)).toBe('2.0 Ko')
    expect(formatBytes(10 * 1024 * 1024)).toBe('10 Mo')
  })
})

describe('basename', () => {
  it('supports unix and windows paths', () => {
    expect(basename('/Users/me/Pictures')).toBe('Pictures')
    expect(basename('C:\\Users\\me\\Pictures')).toBe('Pictures')
  })
})
