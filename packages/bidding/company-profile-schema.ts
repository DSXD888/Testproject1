import type { CompanyProfile } from './bid-scope-types';

export type { CompanyProfile };

export type CompanyProfileValidationError =
  | { kind: 'missing_required_field'; field: string }
  | { kind: 'cert_expired'; certType: string; expiryDate: string }
  | { kind: 'insurance_expired'; insuranceType: string; expiryDate: string };

export function validateCompanyProfile(
  profile: CompanyProfile,
  now: Date,
): CompanyProfileValidationError[] {
  const errors: CompanyProfileValidationError[] = [];

  requireText(errors, profile.companyName, 'companyName');
  requireText(errors, profile.registrationNumber, 'registrationNumber');
  requireText(errors, profile.address?.line1, 'address.line1');
  requireText(errors, profile.address?.city, 'address.city');
  requireText(errors, profile.address?.postcode, 'address.postcode');
  requireText(errors, profile.address?.country, 'address.country');
  requireText(errors, profile.primaryContact?.name, 'primaryContact.name');
  requireText(errors, profile.primaryContact?.email, 'primaryContact.email');
  requireText(errors, profile.primaryContact?.phone, 'primaryContact.phone');

  if (!Array.isArray(profile.certifications) || profile.certifications.length === 0) {
    errors.push({ kind: 'missing_required_field', field: 'certifications' });
  }

  for (const [index, cert] of (profile.certifications ?? []).entries()) {
    requireText(errors, cert.type, `certifications[${index}].type`);
    requireText(errors, cert.number, `certifications[${index}].number`);
    requireText(errors, cert.expiryDate, `certifications[${index}].expiryDate`);
    requireText(errors, cert.issuingBody, `certifications[${index}].issuingBody`);

    if (hasText(cert.expiryDate) && expiryDateHasPassed(cert.expiryDate, now)) {
      errors.push({
        kind: 'cert_expired',
        certType: hasText(cert.type) ? cert.type : 'unknown',
        expiryDate: cert.expiryDate,
      });
    }
  }

  if (!Array.isArray(profile.insurancePolicies) || profile.insurancePolicies.length === 0) {
    errors.push({ kind: 'missing_required_field', field: 'insurancePolicies' });
  }

  for (const [index, insurance] of (profile.insurancePolicies ?? []).entries()) {
    requireText(errors, insurance.type, `insurancePolicies[${index}].type`);
    requireText(errors, insurance.provider, `insurancePolicies[${index}].provider`);
    requireText(errors, insurance.policyNumber, `insurancePolicies[${index}].policyNumber`);
    requirePositiveNumber(
      errors,
      insurance.coverAmountPence,
      `insurancePolicies[${index}].coverAmountPence`,
    );
    requireText(errors, insurance.expiryDate, `insurancePolicies[${index}].expiryDate`);

    if (hasText(insurance.expiryDate) && expiryDateHasPassed(insurance.expiryDate, now)) {
      errors.push({
        kind: 'insurance_expired',
        insuranceType: hasText(insurance.type) ? insurance.type : 'unknown',
        expiryDate: insurance.expiryDate,
      });
    }
  }

  return errors;
}

export function isCompanyProfileCurrent(
  profile: CompanyProfile,
  now: Date = new Date(),
): boolean {
  return validateCompanyProfile(profile, now).length === 0;
}

function requireText(
  errors: CompanyProfileValidationError[],
  value: string | undefined,
  field: string,
): void {
  if (!hasText(value)) {
    errors.push({ kind: 'missing_required_field', field });
  }
}

function requirePositiveNumber(
  errors: CompanyProfileValidationError[],
  value: number,
  field: string,
): void {
  if (!Number.isFinite(value) || value <= 0) {
    errors.push({ kind: 'missing_required_field', field });
  }
}

function hasText(value: string | undefined): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}

function expiryDateHasPassed(expiryDate: string, now: Date): boolean {
  const expiryTimestamp = parseExpiryTimestamp(expiryDate);

  if (expiryTimestamp === null) {
    return true;
  }

  return expiryTimestamp < now.getTime();
}

function parseExpiryTimestamp(expiryDate: string): number | null {
  const trimmed = expiryDate.trim();
  const dateOnlyMatch = /^(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})$/.exec(trimmed);

  if (dateOnlyMatch?.groups) {
    const year = Number(dateOnlyMatch.groups.year);
    const month = Number(dateOnlyMatch.groups.month);
    const day = Number(dateOnlyMatch.groups.day);
    const timestamp = Date.UTC(year, month - 1, day, 23, 59, 59, 999);
    const parsed = new Date(timestamp);

    if (
      parsed.getUTCFullYear() !== year ||
      parsed.getUTCMonth() !== month - 1 ||
      parsed.getUTCDate() !== day
    ) {
      return null;
    }

    return Number.isNaN(timestamp) ? null : timestamp;
  }

  const timestamp = new Date(trimmed).getTime();

  return Number.isNaN(timestamp) ? null : timestamp;
}
