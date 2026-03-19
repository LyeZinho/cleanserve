# Static Blacklist

The StaticBlacklist layer prevents access to sensitive configuration files, version control data, and development tool artifacts. It also restricts the execution of scripts in common upload directories.

## Blocked Files

The following files and directories are blocked from public access. These matches are case-insensitive.

*   **Environment Files**: `.env`, `.env.local`, `.env.*.local`, `.env.example`
*   **Version Control**: `.git`, `.gitignore`, `.gitattributes`, `.github`, `.gitlab-ci.yml`
*   **CI/CD**: `.circleci`, `.travis.yml`, `dockerfile`, `.dockerignore`, `docker-compose.yml`
*   **Dependencies**: `composer.json`, `composer.lock`, `package.json`, `package-lock.json`, `yarn.lock`, `.npmrc`, `.yarnrc`, `gemfile`, `rakefile`
*   **Build Tools**: `webpack.config.js`, `vite.config.js`, `tsconfig.json`, `makefile`, `setup.py`, `setup.cfg`
*   **Configuration**: `.htaccess`, `.editorconfig`, `.prettierrc`, `.eslintrc`, `phpunit.xml`, `.phpunit.xml`, `pytest.ini`
*   **Documentation**: `readme.md`, `license`, `changelog.md`
*   **Reserved Directories**: `.well-known`

## Upload Directory Protection

CleanServe automatically detects common upload directories and restricts the types of files that can be served from them. This prevents attackers from uploading and executing malicious scripts.

### Detected Upload Paths

*   `/uploads/`
*   `/upload/`
*   `/tmp/`
*   `/temp/`
*   `/user_files/`
*   `/files/`

### Forbidden Extensions

The following extensions are blocked when requested from an upload directory:

*   **Scripting**: `php`, `php3-8`, `phtml`, `phar`, `inc`, `pl`, `py`, `jsp`, `asp`, `aspx`, `cgi`
*   **Executables**: `exe`, `sh`, `bat`, `cmd`, `com`
*   **Compiled/Binary**: `bin`, `app`, `jar`, `war`, `class`, `o`, `so`, `dll`, `dylib`

## Error Response

Any request for a blocked file or a forbidden extension in an upload directory results in a `403 Forbidden` status code with a JSON body.

```json
{
  "error": "forbidden",
  "message": "Access to this resource is prohibited for security reasons."
}
```

## Related Documentation

*   [Path Traversal](path-traversal.md)
*   [Security Overview](overview.md)
