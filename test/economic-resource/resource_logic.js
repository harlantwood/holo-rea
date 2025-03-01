const {
  mockAgentId,
  mockIdentifier,
  mockAddress,
  buildConfig,
  buildRunner,
  buildPlayer,
} = require('../init')

const runner = buildRunner()

const config = buildConfig()

const testEventProps = {
  provider: mockAgentId(false),
  receiver: mockAgentId(false),
  hasPointInTime: '2019-11-19T04:29:55.056Z',
}

runner.registerScenario('EconomicResource & EconomicEvent record interactions', async (s, t) => {
  const { cells: [observation, specification] } = await buildPlayer(s, config, ['observation', 'specification'])

  // SCENARIO: write initial records
  const resourceUnitId = mockIdentifier(false)

  const processSpecification = {
    name: 'test process specification A',
  }
  const psResp = await specification.call('process_specification', 'create_process_specification', { process_specification: processSpecification })
  await s.consistency()
  t.ok(psResp.processSpecification && psResp.processSpecification.id, 'process spec 1 created successfully')
  const pSpecId = psResp.processSpecification.id

  const processSpecification2 = {
    name: 'test process specification B',
  }
  const psResp2 = await specification.call('process_specification', 'create_process_specification', { process_specification: processSpecification2 })
  await s.consistency()
  t.ok(psResp2.processSpecification && psResp2.processSpecification.id, 'process spec 2 created successfully')
  const pSpecId2 = psResp2.processSpecification.id

  const resourceSpecification = {
    name: 'test resource specification',
    defaultUnitOfEffort: resourceUnitId,
  }
  const rsResp2 = await specification.call('resource_specification', 'create_resource_specification', { resource_specification: resourceSpecification })
  await s.consistency()
  t.ok(rsResp2.resourceSpecification && rsResp2.resourceSpecification.id, 'resource spec created successfully')
  const resourceSpecificationId = rsResp2.resourceSpecification.id

  const process = {
    name: 'test process for linking logic',
    basedOn: pSpecId,
  }
  const pResp = await observation.call('process', 'create_process', { process })
  await s.consistency()
  t.ok(pResp.process && pResp.process.id, 'process 1 created successfully')
  const processId = pResp.process.id

  const process2 = {
    name: 'test process for specification stage tracking',
    basedOn: pSpecId2,
  }
  const pResp2 = await observation.call('process', 'create_process', { process: process2 })
  await s.consistency()
  t.ok(pResp2.process && pResp2.process.id, 'process 2 created successfully')
  const processId2 = pResp2.process.id

  const inputEvent = {
    note: 'test resource instantiation event',
    action: 'raise',
    resourceClassifiedAs: ['http://www.productontology.org/doc/Apple.ttl'],
    resourceQuantity: { hasNumericalValue: 8, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  const inputResource = {
    note: 'test resource observed in inventory',
    conformsTo: resourceSpecificationId,
  }
  const cResp1 = await observation.call('economic_event', 'create_economic_event', { event: inputEvent, new_inventoried_resource: inputResource })
  await s.consistency()

  const inputEventDest = {
    note: 'input destination inventory for move event test',
    action: 'raise',
    resourceClassifiedAs: ['http://www.productontology.org/doc/Apple.ttl'],
    resourceQuantity: { hasNumericalValue: 0, hasUnit: resourceUnitId },
    resourceConformsTo: resourceSpecificationId,
    ...testEventProps,
  }
  const inputResourceDest = {
    note: 'destination resource for move target',
  }
  const dResp = await observation.call('economic_event', 'create_economic_event', { event: inputEventDest, new_inventoried_resource: inputResourceDest })
  await s.consistency()
  t.ok(dResp.economicEvent, 'destination inventory created successfully')
  const destResourceId = dResp.economicResource.id
  const destResource = dResp.economicResource

  const event = cResp1.economicEvent
  const resource = cResp1.economicResource
  t.ok(event.id, 'event created successfully')
  t.ok(resource.id, 'resource created successfully')
  t.deepEqual(event.resourceInventoriedAs, resource.id, 'resource event link OK')
  t.equal(resource.accountingQuantity.hasNumericalValue, 8, 'resource initial quantity OK')
  const resourceId = resource.id

  // SCENARIO: resource field initialisation
  t.deepEqual(resource.unitOfEffort, resourceUnitId, 'unitOfEffort is set from the resource ResourceSpecification\'s unit of effort')
  t.deepEqual(destResource.unitOfEffort, resourceUnitId, 'unitOfEffort is set from the event ResourceSpecification\'s unit of effort')
  t.deepEqual(event.resourceClassifiedAs, resource.classifiedAs, 'classification is set from the linked event\'s resource classifications')

  // SCENARIO: resource move events
  let newLocation = mockAddress(false)
  let newEvent = {
    resourceInventoriedAs: resourceId,
    toResourceInventoriedAs: destResourceId,
    action: 'move',
    atLocation: newLocation,
    resourceQuantity: { hasNumericalValue: 1, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  let eventResp = await observation.call('economic_event', 'create_economic_event', { event: newEvent })
  await s.consistency()
  t.ok(eventResp.economicEvent, 'appending move event OK')

  let readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  let readResource = readResp.economicResource
  t.deepEqual(readResource.currentLocation, newLocation, 'MOVE events update the resource location if a new location is provided')

  // :TODO: test resource quantities if resourceInventoriedAs and toResourceInventoriedAs are the same. Output from create function is wrong as of 2019-12-03

  // SCENARIO: resource state
  newEvent = {
    resourceInventoriedAs: resourceId,
    action: 'pass',
    outputOf: processId,
    resourceQuantity: { hasNumericalValue: 8, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  eventResp = await observation.call('economic_event', 'create_economic_event', { event: newEvent })
  await s.consistency()
  t.ok(eventResp.economicEvent, 'appending event OK')
  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  readResource = readResp.economicResource
  t.ok(readResource.id, 'resource retrieval OK')
  t.equal(readResource.state, 'pass', 'state should be set to initial action if creating event is PASS or FAIL')

  // SCENARIO: resource stage
  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  readResource = readResp.economicResource
  t.deepEqual(readResource.stage, pSpecId, 'stage should be set to the ProcessSpecification of the output process of the event')

  // SCENARIO: resource math basics
  newEvent = {
    resourceInventoriedAs: resourceId,
    action: 'raise',
    resourceQuantity: { hasNumericalValue: 8, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  eventResp = await observation.call('economic_event', 'create_economic_event', { event: newEvent })
  await s.consistency()
  t.ok(eventResp.economicEvent, 'appending event OK')

  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  readResource = readResp.economicResource
  t.ok(readResource.id, 'resource retrieval OK')
  t.deepEqual(readResource.accountingQuantity, { hasNumericalValue: 15, hasUnit: resourceUnitId }, 'incrementing events increase the accounting quantity of a resource')
  t.deepEqual(readResource.onhandQuantity, { hasNumericalValue: 15, hasUnit: resourceUnitId }, 'incrementing events increase the on-hand quantity of a resource')

  newEvent = {
    resourceInventoriedAs: resourceId,
    action: 'lower',
    resourceQuantity: { hasNumericalValue: 2, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  eventResp = await observation.call('economic_event', 'create_economic_event', { event: newEvent })
  await s.consistency()
  t.ok(eventResp.economicEvent, 'appending event OK')

  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  readResource = readResp.economicResource
  t.deepEqual(readResource.accountingQuantity, { hasNumericalValue: 13, hasUnit: resourceUnitId }, 'decrementing events decrease the accounting quantity of a resource')
  t.deepEqual(readResource.onhandQuantity, { hasNumericalValue: 13, hasUnit: resourceUnitId }, 'decrementing events decrease the on-hand quantity of a resource')

  newEvent = {
    resourceInventoriedAs: resourceId,
    action: 'transfer-custody',
    resourceQuantity: { hasNumericalValue: 1, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  eventResp = await observation.call('economic_event', 'create_economic_event', { event: newEvent })
  await s.consistency()
  t.ok(eventResp.economicEvent, 'appending event OK')

  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  readResource = readResp.economicResource
  t.deepEqual(readResource.accountingQuantity, { hasNumericalValue: 13, hasUnit: resourceUnitId }, 'transfer-custody does not update accountingQuantity')
  t.deepEqual(readResource.onhandQuantity, { hasNumericalValue: 12, hasUnit: resourceUnitId }, 'transfer-custody updates onhandQuantity')

  newEvent = {
    resourceInventoriedAs: resourceId,
    action: 'transfer-all-rights',
    resourceQuantity: { hasNumericalValue: 1, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  eventResp = await observation.call('economic_event', 'create_economic_event', { event: newEvent })
  await s.consistency()
  t.ok(eventResp.economicEvent, 'appending event OK')

  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  readResource = readResp.economicResource
  t.deepEqual(readResource.accountingQuantity, { hasNumericalValue: 12, hasUnit: resourceUnitId }, 'transfer-all-rights updates accountingQuantity')
  t.deepEqual(readResource.onhandQuantity, { hasNumericalValue: 12, hasUnit: resourceUnitId }, 'transfer-all-rights does not update onhandQuantity')

  // SCENARIO: secondary resource for inventory transfer tests
  const inputEvent2 = {
    note: 'event to instantiate receiving resource',
    action: 'raise',
    resourceClassifiedAs: ['http://www.productontology.org/doc/Apple.ttl'],
    resourceQuantity: { hasNumericalValue: 0, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  const inputResource2 = {
    note: 'receiver test resource',
    conformsTo: resourceSpecificationId,
  }
  const cResp2 = await observation.call('economic_event', 'create_economic_event', { event: inputEvent2, new_inventoried_resource: inputResource2 })
  await s.consistency()
  const event2 = cResp2.economicEvent
  const resource2 = cResp2.economicResource
  t.ok(event2.id, '2nd event created successfully')
  t.ok(resource2.id, '2nd resource created successfully')
  const resourceId2 = resource2.id

  // SCENARIO: resource transfer behaviour
  newEvent = {
    resourceInventoriedAs: resourceId,
    toResourceInventoriedAs: resourceId2,
    action: 'transfer',
    resourceQuantity: { hasNumericalValue: 3, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  eventResp = await observation.call('economic_event', 'create_economic_event', { event: newEvent })
  await s.consistency()
  t.ok(eventResp.economicEvent, 'appending event OK')

  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  readResource = readResp.economicResource
  t.deepEqual(readResource.accountingQuantity, { hasNumericalValue: 9, hasUnit: resourceUnitId }, 'transfer events decrease the accounting quantity of the sending resource')
  t.deepEqual(readResource.onhandQuantity, { hasNumericalValue: 9, hasUnit: resourceUnitId }, 'transfer events decrease the onhand quantity of the sending resource')

  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId2 })
  readResource = readResp.economicResource
  t.deepEqual(readResource.accountingQuantity, { hasNumericalValue: 3, hasUnit: resourceUnitId }, 'transfer events increase the accounting quantity of the receiving resource')
  t.deepEqual(readResource.onhandQuantity, { hasNumericalValue: 3, hasUnit: resourceUnitId }, 'transfer events increase the onhand quantity of the receiving resource')

  // SCENARIO: field update tests for event bindings
  newEvent = {
    resourceInventoriedAs: resourceId,
    action: 'fail',
    outputOf: processId2,
    resourceQuantity: { hasNumericalValue: 3, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  eventResp = await observation.call('economic_event', 'create_economic_event', { event: newEvent })
  await s.consistency()
  t.ok(eventResp.economicEvent, 'appending event OK')

  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  readResource = readResp.economicResource
  t.equal(readResource.state, 'fail', 'should take on the last PASS | FAIL event action as its state')
  t.deepEqual(readResource.stage, pSpecId2, 'should take on the stage of the most recent event\'s related output ProcessSpecification')

  newEvent = {
    resourceInventoriedAs: resourceId,
    resourceClassifiedAs: ['http://www.productontology.org/doc/Manure_spreader.ttl'],
    action: 'raise',
    resourceQuantity: { hasNumericalValue: 1, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  eventResp = await observation.call('economic_event', 'create_economic_event', { event: newEvent })
  await s.consistency()
  t.ok(eventResp.economicEvent, 'appending event OK')

  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  readResource = readResp.economicResource
  t.deepEqual(readResource.classifiedAs,
    ['http://www.productontology.org/doc/Apple.ttl', 'http://www.productontology.org/doc/Manure_spreader.ttl'],
    'creating an associated event with a new ResourceClassification type appends the classification to the resource\'s existing classifications'
  )

  newEvent = {
    resourceInventoriedAs: resourceId,
    resourceClassifiedAs: ['http://www.productontology.org/doc/Manure_spreader.ttl'],
    action: 'raise',
    resourceQuantity: { hasNumericalValue: 1, hasUnit: resourceUnitId },
    ...testEventProps,
  }
  eventResp = await observation.call('economic_event', 'create_economic_event', { event: newEvent })
  await s.consistency()
  t.ok(eventResp.economicEvent, 'appending event OK')

  readResp = await observation.call('economic_resource', 'get_economic_resource', { address: resourceId })
  readResource = readResp.economicResource
  t.deepEqual(readResource.classifiedAs,
    ['http://www.productontology.org/doc/Apple.ttl', 'http://www.productontology.org/doc/Manure_spreader.ttl'],
    'multiple events with the same ResourceClassification yield only 1 occurence of the classification in the resource data'
  )
})

runner.run()
