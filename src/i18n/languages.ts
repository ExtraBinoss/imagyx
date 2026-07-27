export interface Language {
  code: string
  nativeName: string
  name: string
}

export const languages: Language[] = [
  { code: 'en', nativeName: 'English', name: 'English' },
  { code: 'fr', nativeName: 'Français', name: 'French' },
  { code: 'es', nativeName: 'Español', name: 'Spanish' },
  { code: 'de', nativeName: 'Deutsch', name: 'German' },
  { code: 'it', nativeName: 'Italiano', name: 'Italian' },
  { code: 'pt', nativeName: 'Português', name: 'Portuguese' },
  { code: 'ja', nativeName: '日本語', name: 'Japanese' },
  { code: 'zh', nativeName: '简体中文', name: 'Chinese (Simplified)' },
  { code: 'ar', nativeName: 'العربية', name: 'Arabic' },
  { code: 'ru', nativeName: 'Русский', name: 'Russian' },
]
