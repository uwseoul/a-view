const test = require('node:test')
const assert = require('node:assert/strict')
const { classifyStatus } = require('../server/stall-detector')

const now = new Date('2026-03-30T07:00:45.000Z').getTime()

test('running before 30 seconds', () => {
  assert.equal(classifyStatus({ lastActivityAt: '2026-03-30T07:00:20.000Z', now }), 'running')
})

test('delayed at 30 seconds', () => {
  assert.equal(classifyStatus({ lastActivityAt: '2026-03-30T07:00:15.000Z', now }), 'delayed')
})

test('delayed at 44 seconds', () => {
  assert.equal(classifyStatus({ lastActivityAt: '2026-03-30T07:00:01.000Z', now }), 'delayed')
})

test('stalled at 45 seconds', () => {
  assert.equal(classifyStatus({ lastActivityAt: '2026-03-30T07:00:00.000Z', now }), 'stalled')
})

test('explicit failure wins', () => {
  assert.equal(classifyStatus({ explicitStatus: 'failed', lastActivityAt: '2026-03-30T07:00:30.000Z', now }), 'failed')
})
