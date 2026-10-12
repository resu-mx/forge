import { describe, expect, test } from 'bun:test'
import type { ResumeTaglineState } from '@forge/sdk'
import { taglineView } from './tagline-view'

const state = (s: Partial<ResumeTaglineState>): ResumeTaglineState => ({
  generated_tagline: null, tagline_override: null, resolved: '', has_override: false, ...s,
})

describe('taglineView', () => {
  test('no state yet: header fallback, no badge, no reset, empty edit box', () => {
    expect(taglineView(null, 'Platform Engineer')).toEqual({
      display: 'Platform Engineer', badge: null, canReset: false, editSeed: '',
    })
  })

  test('an override wins', () => {
    const s = state({ generated_tagline: 'SRE · Kubernetes', tagline_override: 'Platform · SRE', resolved: 'Platform · SRE', has_override: true })
    expect(taglineView(s, 'x')).toEqual({ display: 'Platform · SRE', badge: 'override', canReset: true, editSeed: 'Platform · SRE' })
  })

  test('generated only: AUTO, no reset, edit box seeded with the generated tagline', () => {
    const s = state({ generated_tagline: 'SRE · Kubernetes', resolved: 'SRE · Kubernetes' })
    expect(taglineView(s, 'x')).toEqual({ display: 'SRE · Kubernetes', badge: 'auto', canReset: false, editSeed: 'SRE · Kubernetes' })
  })

  test('nothing resolved: falls back to the header tagline', () => {
    expect(taglineView(state({}), 'Platform Engineer').display).toBe('Platform Engineer')
  })

  test('an empty generated tagline is not AUTO', () => {
    expect(taglineView(state({ generated_tagline: '' }), null).badge).toBeNull()
  })

  // resu-mx/forge#72: a blank override is no override. The server's `resolved` and `has_override`
  // already say so; the edit box must not open on the whitespace either.
  test('a whitespace-only override: AUTO, no reset, and the edit box opens on the generated tagline', () => {
    const s = state({ generated_tagline: 'SRE · Kubernetes', tagline_override: '   ', resolved: 'SRE · Kubernetes', has_override: false })
    expect(taglineView(s, 'x')).toEqual({ display: 'SRE · Kubernetes', badge: 'auto', canReset: false, editSeed: 'SRE · Kubernetes' })
  })
})
