import common from './common'
import search from './search'
import spotlight from './spotlight'
import sidebar from './sidebar'
import onboarding from './onboarding'
import indexing from './indexing'
import settings from './settings'

const messages: Record<string, string> = {
  ...common,
  ...search,
  ...spotlight,
  ...sidebar,
  ...onboarding,
  ...indexing,
  ...settings,
}

export default messages
