# Refine Spec Command

Refines an incomplete or unfocused specification by addressing validation gaps, splitting if needed, or enhancing details.

Arguments: $ARGUMENTS

## Usage

```
/prodigy-refine-spec <spec-identifier> <original-description> [--gaps <validation-gaps-json>]
```

Examples:
- `/prodigy-refine-spec 01 "Add user authentication" --gaps ${validation.gaps}`
- `/prodigy-refine-spec 02 "Implement caching layer"`

## What This Command Does

1. **Receives Validation Feedback**
   - Gets gaps from spec completeness validation
   - Understands original intent from description
   - Identifies specific improvements needed

2. **Refines Existing Spec**
   - Adds missing sections
   - Clarifies vague requirements
   - Makes acceptance criteria testable
   - Enhances technical details

3. **Splits Spec if Needed**
   - Creates multiple focused specs from bloated one
   - Maintains traceability between specs
   - Ensures each spec has single responsibility

4. **Commits Changes**
   - Updates existing spec or creates new ones
   - Commits with clear message about refinements

## Execution Process

### Step 1: Parse Input and Analyze Gaps

- Extract spec identifier from $ARGUMENTS
- Extract original description
- Parse --gaps parameter with validation data
- Read current spec file from `specs/{number}-*.md`

### Step 2: Determine Refinement Strategy

Based on gaps, choose approach:

#### A. Enhancement Strategy (completion < 90%, focused spec)
- Add missing sections
- Clarify requirements
- Improve acceptance criteria
- Add technical details

#### B. Splitting Strategy (not focused, too broad)
- Identify logical boundaries
- Create separate spec files
- Maintain dependencies between specs
- Delete original bloated spec

#### C. Complete Rewrite (completion < 50%)
- Start fresh with clear structure
- Ensure all sections present
- Focus on single objective

### Step 3: Enhancement Implementation

For specs that need enhancement:

#### Missing Sections
Add any missing required sections:
- Context and background
- Clear objectives
- Functional/non-functional requirements
- Testable acceptance criteria
- Technical implementation details
- Testing strategy

#### Vague Requirements
Transform vague statements into specific ones:
```markdown
# Before
- System should be fast
- UI should be user-friendly

# After
- API responses must return within 200ms for 95% of requests
- All interactive elements must have visible focus indicators
- Forms must show inline validation errors within 100ms
```

#### Testability Issues
Make all acceptance criteria verifiable:
```markdown
# Before
- [ ] Good performance
- [ ] Secure implementation

# After
- [ ] Page load time under 2 seconds on 3G connection
- [ ] All passwords hashed using bcrypt with min cost factor 10
- [ ] Session tokens expire after 24 hours of inactivity
```

### Step 4: Spec Splitting Implementation

When spec needs splitting:

1. **Analyze Scope**
   - Identify distinct features/components
   - Group related requirements
   - Define clear boundaries

2. **Create New Spec Files**
   - Generate next spec numbers
   - Create focused spec for each component
   - Each spec gets 3-8 acceptance criteria

3. **Establish Dependencies**
   - Note dependencies in frontmatter
   - Reference related specs
   - Maintain implementation order

4. **Example Split**
   Original: "User authentication and profile management"

   Split into:
   - `specs/03-user-authentication.md` - Login, logout, session management
   - `specs/04-user-profiles.md` - Profile CRUD, avatar upload
   - `specs/05-password-reset.md` - Reset flow, email verification

### Step 5: File Operations

#### For Enhancement
- Use Edit tool to update existing spec
- Preserve spec number and structure
- Update frontmatter if needed

#### For Splitting
- Create new spec files with Write tool
- Each gets unique number (highest + 1, +2, etc.)
- Delete original spec file after split
- Update any references in other specs

### Step 6: Commit Changes

Create git commit with all changes:

```bash
# For enhancement
git add specs/
git commit -m "refine: enhance spec {NUMBER} for completeness

- Added missing technical details
- Clarified acceptance criteria
- Made all requirements testable"

# For splitting
git add specs/
git rm specs/{original}.md
git commit -m "refine: split spec {NUMBER} into focused components

- Created specs {NEW_NUMBERS} from original
- Each spec now has single responsibility
- Maintained dependencies and relationships"
```

**Note**: Keep commits clean without attribution text.

## Gap Resolution Patterns

### Missing Technical Details
```markdown
## Technical Details

### Implementation Approach
- Use repository pattern for data access
- Implement service layer for business logic
- Apply dependency injection for testability

### Architecture Changes
- Add AuthenticationService to handle login/logout
- Create SessionManager for token lifecycle
- Integrate with existing UserRepository

### Data Structures
```rust
pub struct AuthToken {
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
}
```

### APIs and Interfaces
```rust
pub trait AuthService {
    async fn login(&self, credentials: LoginRequest) -> Result<AuthToken>;
    async fn logout(&self, token: &str) -> Result<()>;
    async fn validate(&self, token: &str) -> Result<User>;
}
```
```

### Vague Acceptance Criteria
```markdown
# Before
- [ ] Users can log in

# After
- [ ] Users can log in with email/password via POST /api/auth/login
- [ ] Successful login returns JWT token valid for 24 hours
- [ ] Failed login returns 401 with error message after 3 attempts
- [ ] Login attempts are rate-limited to 5 per minute per IP
```

### Scope Too Broad
```markdown
# Original Spec 01: Complete User System

# Becomes:

## Spec 01: User Authentication
- JWT-based authentication
- Login/logout endpoints
- Session management

## Spec 02: User Registration
- Registration endpoint
- Email verification
- Welcome email

## Spec 03: Password Management
- Password reset flow
- Change password endpoint
- Password strength validation

## Spec 04: User Profiles
- Profile CRUD operations
- Avatar upload
- Profile visibility settings
```

## Output Format

After refinement, output status:

```json
{
  "refinement_status": "complete",
  "action_taken": "enhanced|split|rewritten",
  "specs_modified": ["01-user-authentication.md"],
  "specs_created": ["02-user-registration.md", "03-password-reset.md"],
  "specs_deleted": ["01-user-system.md"],
  "improvements": [
    "Added technical implementation details",
    "Made all acceptance criteria testable",
    "Split into 3 focused specs"
  ]
}
```

## Quality Standards

### For Enhanced Specs
- All sections must be complete
- Requirements must be specific
- Acceptance criteria must be testable
- Technical approach must be clear

### For Split Specs
- Each spec has single responsibility
- 3-8 acceptance criteria per spec
- Clear dependencies noted
- Can be implemented in 1-2 days

### Commit Standards
- Clear commit messages
- All changes in single commit
- Reference spec numbers
- No attribution text

## Error Handling

Handle these cases:
- Spec file not found
- Invalid gaps JSON
- Unable to determine split boundaries
- Conflicting spec numbers

## Important Notes

1. **Preserve intent** - Keep original functionality when splitting
2. **Maintain focus** - Each spec should do one thing well
3. **Ensure testability** - All criteria must be verifiable
4. **Clean commits** - No attribution text in commits
5. **Update dependencies** - Fix references when splitting
6. **Delete originals** - Remove bloated specs after splitting
7. **Atomic operations** - All changes in one commit