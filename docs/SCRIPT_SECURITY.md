# TypeScript script-host security

Server game modules выполняются в отдельном Node.js process. Environment очищается, filesystem permissions ограничиваются read-only путями модуля и SDK, запись в filesystem не разрешается, команды проходят Rust validation, а tick ограничен timeout и command quota.

Node permission model является дополнительной защитой, но не заменяет доверенную модель расширений. Публичный сервер должен запускать только проверенные и подписанные game modules. Недоверенный пользовательский код требует отдельного OS container/VM sandbox.
