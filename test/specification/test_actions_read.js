// :NOTE: this is a minimal test- actual actions are tested in Rust unit tests

const {
  buildConfig,
  buildRunner,
  buildPlayer,
} = require('../init')

const runner = buildRunner()

const config = buildConfig()

runner.registerScenario('Built-in action API', async (s, t) => {
  const alice = await buildPlayer(s, config, ['specification'])

  const queryAllResp = await alice.graphQL(`
    {
      actions {
        id
      }
    }
  `, {})

  t.equal(queryAllResp.data.actions.length, 18, 'all action builtins present')

  const getResp = await alice.graphQL(`
    query($id: ID!) {
      action(id: $id) {
        id
        label
        resourceEffect
        inputOutput
        pairsWith
      }
    }
  `, {
    id: 'raise',
  })

  t.deepEqual(getResp.data.action, {
    id: 'raise',
    label: 'raise',
    resourceEffect: 'increment',
    inputOutput: 'notApplicable',
    pairsWith: 'notApplicable',
  }, 'record read OK')
})

runner.run()
