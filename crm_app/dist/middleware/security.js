const rateLimit = require('express-rate-limit');
const crypto = require('crypto');

// 1. محدد معدل الطلبات (Rate Limiting)
const limiter = rateLimit({
  windowMs: 15 * 60 * 1000,
  max: 100,
  standardHeaders: true,
  legacyHeaders: false,
  message: 'لقد تجاوزت الحد المسموح من الطلبات. يرجى المحاولة لاحقاً.'
});

// 2. التحقق من توكن CSRF المخصص والمستقر
const authLimiter = rateLimit({
  windowMs: 15 * 60 * 1000,
  max: 20,
  standardHeaders: true,
  legacyHeaders: false,
  message: 'Too many authentication attempts. Please retry later.'
});

const apiLimiter = rateLimit({
  windowMs: 60 * 1000,
  max: 120,
  standardHeaders: true,
  legacyHeaders: false,
  message: { error: 'API rate limit exceeded. Please retry later.' }
});

const getCookie = (req, name) => {
  const cookies = req.headers.cookie;
  if (!cookies) return null;
  const match = cookies.match(new RegExp('(^|;)\\s*' + name + '\\s*=\\s*([^;]+)'));
  return match ? decodeURIComponent(match[2]) : null;
};

const csrfProtection = (req, res, next) => {
  let cookieToken = getCookie(req, 'csrfToken');
  if (!cookieToken) {
    cookieToken = crypto.randomBytes(32).toString('hex');
    res.cookie('csrfToken', cookieToken, { httpOnly: true, secure: process.env.NODE_ENV === 'production', sameSite: 'lax' });
  }

  if (req.session) {
    req.session.csrfToken = cookieToken;
  }

  if (req.method === 'POST') {
    const token = req.body._csrf || req.headers['x-csrf-token'];
    if (!token || token !== cookieToken) {
      return res.status(403).send('CSRF validation failed. Unauthorized request.');
    }
  }
  next();
};

function sanitizeValue(value) {
  if (typeof value === 'string') {
    return value
      .replace(/<script[\s\S]*?>[\s\S]*?<\/script>/gi, '')
      .replace(/\son\w+\s*=\s*(['"]).*?\1/gi, '')
      .replace(/javascript:/gi, '');
  }
  if (Array.isArray(value)) return value.map(sanitizeValue);
  if (value && typeof value === 'object') {
    for (const key of Object.keys(value)) {
      value[key] = sanitizeValue(value[key]);
    }
  }
  return value;
}

const inputSanitizer = (req, _res, next) => {
  req.body = sanitizeValue(req.body || {});
  req.query = sanitizeValue(req.query || {});
  req.params = sanitizeValue(req.params || {});
  next();
};

module.exports = {
  limiter,
  authLimiter,
  apiLimiter,
  csrfProtection,
  inputSanitizer
};
