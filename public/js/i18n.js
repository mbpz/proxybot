/**
 * ProxyBot Landing Page i18n
 * Detects language, loads translation JSON, replaces data-i18n elements.
 */
(function() {
  'use strict';

  const STORAGE_KEY = 'proxybot_lang';
  const SUPPORTED = ['en', 'zh'];
  const DEFAULT_LANG = 'en';

  let currentLang = DEFAULT_LANG;
  let translations = {};

  function detectLanguage() {
    // 1. Check localStorage for explicit user choice
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored && SUPPORTED.includes(stored)) return stored;

    // 2. Check browser language
    const browserLang = navigator.language || '';
    if (browserLang.startsWith('zh')) return 'zh';
    if (browserLang.startsWith('en')) return 'en';

    // 3. Default
    return DEFAULT_LANG;
  }

  function loadTranslations(lang) {
    return fetch('/locales/' + lang + '.json')
      .then(function(response) {
        if (!response.ok) throw new Error('Failed to load ' + lang);
        return response.json();
      })
      .then(function(data) {
        translations = data;
        currentLang = lang;
        localStorage.setItem(STORAGE_KEY, lang);
        return data;
      })
      .catch(function(err) {
        console.warn('i18n: failed to load', lang, err);
        // Fallback to embedded English
        if (lang !== DEFAULT_LANG) {
          return loadTranslations(DEFAULT_LANG);
        }
      });
  }

  // Get nested translation by dot-notation key
  function getTranslation(key) {
    var parts = key.split('.');
    var value = translations;
    for (var i = 0; i < parts.length; i++) {
      if (value === undefined || value === null) return key;
      value = value[parts[i]];
    }
    return value !== undefined && value !== null ? value : key;
  }

  // Recursively translate an element and its children
  function translateElement(el) {
    var key = el.getAttribute('data-i18n');
    if (key) {
      var text = getTranslation(key);
      if (text !== key) {
        el.textContent = text;
      }
    }
    // Handle data-i18n-* for attribute translation
    var attrs = el.attributes;
    for (var i = attrs.length - 1; i >= 0; i--) {
      var attr = attrs[i];
      if (attr.name.startsWith('data-i18n-')) {
        var targetAttr = attr.name.replace('data-i18n-', '');
        var transKey = attr.value;
        var transValue = getTranslation(transKey);
        if (transValue !== transKey) {
          el.setAttribute(targetAttr, transValue);
        }
        el.removeAttribute(attr.name);
      }
    }
  }

  function translatePage() {
    // Translate all elements with data-i18n
    var elements = document.querySelectorAll('[data-i18n]');
    for (var i = 0; i < elements.length; i++) {
      translateElement(elements[i]);
    }

    // Handle special translation targets
    // Hero demo lines use nested structure
    var demoLines = document.querySelectorAll('.demo-line');
    var demoData = translations.hero_demo;
    if (demoData && demoLines.length > 0) {
      // Lines are already populated from the HTML, but we update their content
      // based on the translation structure if needed
    }

    // Update html lang attribute
    document.documentElement.lang = currentLang;
  }

  function switchLanguage(lang) {
    if (!SUPPORTED.includes(lang)) return;
    loadTranslations(lang).then(function() {
      translatePage();
      updateSwitcherUI();
    });
  }

  function updateSwitcherUI() {
    var btns = document.querySelectorAll('.lang-btn');
    for (var i = 0; i < btns.length; i++) {
      var btn = btns[i];
      if (btn.getAttribute('data-lang') === currentLang) {
        btn.classList.add('active');
      } else {
        btn.classList.remove('active');
      }
    }
  }

  // Public API
  window.i18n = {
    currentLang: function() { return currentLang; },
    switch: switchLanguage,
    t: getTranslation
  };

  // Init on DOM ready
  document.addEventListener('DOMContentLoaded', function() {
    var lang = detectLanguage();
    loadTranslations(lang).then(function() {
      translatePage();
      // Set up language switcher buttons
      var btns = document.querySelectorAll('.lang-btn');
      for (var i = 0; i < btns.length; i++) {
        (function(btn) {
          btn.addEventListener('click', function(e) {
            e.preventDefault();
            switchLanguage(btn.getAttribute('data-lang'));
          });
        })(btns[i]);
      }
      updateSwitcherUI();
    });
  });
})();