use std::cell::Cell;
use std::ffi::{CStr, CString};
use std::mem::{MaybeUninit, zeroed};
use std::os::raw::{c_char, c_uint, c_void};
use std::ptr::{copy_nonoverlapping, null, null_mut};
use std::sync::Mutex;

use num_traits::FromPrimitive;
use serde::{Deserialize, Serialize};

use crate::*;
use crate::bindings::capi as ztcore;
use crate::bindings::capi::ZT_CertificateError;

/// Maximum length of a string in a certificate (mostly for the certificate name fields).
pub const CERTIFICATE_MAX_STRING_LENGTH: isize = ztcore::ZT_CERTIFICATE_MAX_STRING_LENGTH as isize;

/// Certificate local trust bit field flag: this certificate self-signs a root CA.
pub const CERTIFICATE_LOCAL_TRUST_FLAG_ROOT_CA: u32 = ztcore::ZT_CERTIFICATE_LOCAL_TRUST_FLAG_ROOT_CA;

/// Certificate local trust bit field flag: this certificate specifies a set of ZeroTier roots.
pub const CERTIFICATE_LOCAL_TRUST_FLAG_ZEROTIER_ROOT_SET: u32 = ztcore::ZT_CERTIFICATE_LOCAL_TRUST_FLAG_ZEROTIER_ROOT_SET;

/// Length of a NIST P-384 unique ID (public key).
pub const CERTIFICATE_UNIQUE_ID_TYPE_NIST_P_384_SIZE: u32 = ztcore::ZT_CERTIFICATE_UNIQUE_ID_TYPE_NIST_P_384_SIZE;

/// Length of a private key corresponding to a NIST P-384 unique ID.
pub const CERTIFICATE_UNIQUE_ID_TYPE_NIST_P_384_PRIVATE_SIZE: u32 = ztcore::ZT_CERTIFICATE_UNIQUE_ID_TYPE_NIST_P_384_PRIVATE_SIZE;

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct CertificateSerialNo(pub [u8; 48]);

impl CertificateSerialNo {
    /// Create a new empty (all zero) serial number.
    #[inline(always)]
    pub fn new() -> CertificateSerialNo {
        CertificateSerialNo([0; 48])
    }

    pub fn new_from_string(s: &str) -> Result<CertificateSerialNo, ResultCode> {
        let b = hex::decode(s);
        if b.is_err() {
            return Err(ResultCode::ErrorBadParameter);
        }
        return Ok(CertificateSerialNo::from(b.unwrap().as_slice()));
    }
}

impl From<&[u8; 48]> for CertificateSerialNo {
    fn from(a: &[u8; 48]) -> CertificateSerialNo {
        CertificateSerialNo(*a)
    }
}

impl From<&[u8]> for CertificateSerialNo {
    fn from(v: &[u8]) -> CertificateSerialNo {
        let mut l = v.len();
        if l > 48 {
            l = 48;
        }
        let mut s = CertificateSerialNo::new();
        s.0[0..l].copy_from_slice(&v[0..l]);
        s
    }
}

impl ToString for CertificateSerialNo {
    fn to_string(&self) -> String {
        hex::encode(self.0)
    }
}

impl serde::Serialize for CertificateSerialNo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        serializer.serialize_str(self.to_string().as_str())
    }
}

struct CertificateSerialNoVisitor;

impl<'de> serde::de::Visitor<'de> for CertificateSerialNoVisitor {
    type Value = CertificateSerialNo;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("CertificateSerialNoVisitor value in string form")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E> where E: serde::de::Error {
        let id = CertificateSerialNo::new_from_string(s);
        if id.is_err() {
            return Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(s), &self));
        }
        return Ok(id.ok().unwrap() as Self::Value);
    }
}

impl<'de> serde::Deserialize<'de> for CertificateSerialNo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_str(CertificateSerialNoVisitor)
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Type of certificate subject unique ID
#[derive(FromPrimitive, ToPrimitive)]
pub enum CertificateUniqueIdType {
    NistP384 = ztcore::ZT_CertificateUniqueIdType_ZT_CERTIFICATE_UNIQUE_ID_TYPE_NIST_P_384 as isize
}

impl CertificateUniqueIdType {
    pub fn new_from_string(s: &str) -> Result<CertificateUniqueIdType, ResultCode> {
        if s.to_ascii_lowercase() == "nistp384" {
            return Ok(CertificateUniqueIdType::NistP384);
        }
        return Err(ResultCode::ErrorBadParameter);
    }
}

impl ToString for CertificateUniqueIdType {
    fn to_string(&self) -> String {
        match *self {
            CertificateUniqueIdType::NistP384 => String::from("NistP384")
        }
    }
}

impl serde::Serialize for CertificateUniqueIdType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        serializer.serialize_str(self.to_string().as_str())
    }
}

struct CertificateUniqueIdTypeVisitor;

impl<'de> serde::de::Visitor<'de> for CertificateUniqueIdTypeVisitor {
    type Value = CertificateUniqueIdType;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("CertificateUniqueIdType value in string form")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E> where E: serde::de::Error {
        let id = CertificateUniqueIdType::new_from_string(s);
        if id.is_err() {
            return Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(s), &self));
        }
        return Ok(id.ok().unwrap() as Self::Value);
    }
}

impl<'de> serde::Deserialize<'de> for CertificateUniqueIdType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_str(CertificateUniqueIdTypeVisitor)
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize)]
pub struct CertificateSubjectUniqueIdSecret {
    pub public: Vec<u8>,
    pub private: Vec<u8>,
    pub type_: CertificateUniqueIdType,
}

const CERTIFICATE_UNIQUE_ID_CREATE_BUF_SIZE: usize = 128;

impl CertificateSubjectUniqueIdSecret {
    pub fn new(t: CertificateUniqueIdType) -> Self {
        let mut unique_id: [u8; CERTIFICATE_UNIQUE_ID_CREATE_BUF_SIZE] = [0; CERTIFICATE_UNIQUE_ID_CREATE_BUF_SIZE];
        let mut unique_id_private: [u8; CERTIFICATE_UNIQUE_ID_CREATE_BUF_SIZE] = [0; CERTIFICATE_UNIQUE_ID_CREATE_BUF_SIZE];
        let mut unique_id_size = CERTIFICATE_UNIQUE_ID_CREATE_BUF_SIZE as c_int;
        let mut unique_id_private_size = CERTIFICATE_UNIQUE_ID_CREATE_BUF_SIZE as c_int;
        let ct: ztcore::ZT_CertificateUniqueIdType = num_traits::ToPrimitive::to_u32(&t).unwrap();
        unsafe {
            if ztcore::ZT_Certificate_newSubjectUniqueId(ct, unique_id.as_mut_ptr() as *mut c_void, &mut unique_id_size, unique_id_private.as_mut_ptr() as *mut c_void, &mut unique_id_private_size) != 0 {
                panic!("fatal internal error: ZT_Certificate_newSubjectUniqueId failed.");
            }
        }
        CertificateSubjectUniqueIdSecret {
            public: Vec::from(&unique_id[0..unique_id_size as usize]),
            private: Vec::from(&unique_id_private[0..unique_id_private_size as usize]),
            type_: num_traits::FromPrimitive::from_u32(ct as u32).unwrap(),
        }
    }
}

implement_to_from_json!(CertificateSubjectUniqueIdSecret);

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

/// Reasons a certificate may be rejected.
#[derive(FromPrimitive, ToPrimitive)]
pub enum CertificateError {
    None = ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_NONE as isize,
    HaveNewerCert = ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_HAVE_NEWER_CERT as isize,
    InvalidFormat = ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_INVALID_FORMAT as isize,
    InvalidIdentity = ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_INVALID_IDENTITY as isize,
    InvalidPrimarySignature = ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_INVALID_PRIMARY_SIGNATURE as isize,
    InvalidChain = ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_INVALID_CHAIN as isize,
    InvalidComponentSignature = ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_INVALID_COMPONENT_SIGNATURE as isize,
    InvalidUniqueIdProof = ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_INVALID_UNIQUE_ID_PROOF as isize,
    MissingRequiredFields = ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_MISSING_REQUIRED_FIELDS as isize,
    OutOfValidTimeWindow = ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_OUT_OF_VALID_TIME_WINDOW as isize,
}

impl ToString for CertificateError {
    fn to_string(&self) -> String {
        String::from(
            match self {
                CertificateError::None => "None",
                CertificateError::HaveNewerCert => "HaveNewerCert",
                CertificateError::InvalidFormat => "InvalidFormat",
                CertificateError::InvalidIdentity => "InvalidIdentity",
                CertificateError::InvalidPrimarySignature => "InvalidPrimarySignature",
                CertificateError::InvalidChain => "InvalidChain",
                CertificateError::InvalidComponentSignature => "InvalidComponentSignature",
                CertificateError::InvalidUniqueIdProof => "InvalidUniqueIdProof",
                CertificateError::MissingRequiredFields => "MissingRequiredFields",
                CertificateError::OutOfValidTimeWindow => "OutOfValidTimeWindow"
            }
        )
    }
}

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct CertificateName {
    pub serialNo: String,
    pub commonName: String,
    pub country: String,
    pub organization: String,
    pub unit: String,
    pub locality: String,
    pub province: String,
    pub streetAddress: String,
    pub postalCode: String,
    pub email: String,
    pub url: String,
    pub host: String,
}

impl CertificateName {
    pub(crate) unsafe fn new_from_capi(cn: &ztcore::ZT_Certificate_Name) -> CertificateName {
        unsafe {
            return CertificateName {
                serialNo: cstr_to_string(cn.serialNo.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                commonName: cstr_to_string(cn.commonName.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                country: cstr_to_string(cn.country.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                organization: cstr_to_string(cn.organization.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                unit: cstr_to_string(cn.unit.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                locality: cstr_to_string(cn.locality.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                province: cstr_to_string(cn.province.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                streetAddress: cstr_to_string(cn.streetAddress.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                postalCode: cstr_to_string(cn.postalCode.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                email: cstr_to_string(cn.email.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                url: cstr_to_string(cn.url.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1),
                host: cstr_to_string(cn.host.as_ptr(), CERTIFICATE_MAX_STRING_LENGTH - 1)
            };
        }
    }

    fn str_to_cert_cstr(s: &String, cs: &mut [c_char; 128]) {
        let mut l = s.len();
        if l == 0 {
            cs[0] = 0;
            return;
        }
        if l > 126 {
            l = 126;
        }
        unsafe {
            copy_nonoverlapping(s.as_ptr(), cs.as_mut_ptr() as *mut u8, l);
        }
        cs[l + 1] = 0;
    }

    pub(crate) unsafe fn to_capi(&self) -> ztcore::ZT_Certificate_Name {
        unsafe {
            let mut cn: ztcore::ZT_Certificate_Name = zeroed();
            Self::str_to_cert_cstr(&self.serialNo, &mut cn.serialNo);
            Self::str_to_cert_cstr(&self.commonName, &mut cn.commonName);
            Self::str_to_cert_cstr(&self.country, &mut cn.country);
            Self::str_to_cert_cstr(&self.organization, &mut cn.organization);
            Self::str_to_cert_cstr(&self.unit, &mut cn.unit);
            Self::str_to_cert_cstr(&self.locality, &mut cn.locality);
            Self::str_to_cert_cstr(&self.province, &mut cn.province);
            Self::str_to_cert_cstr(&self.streetAddress, &mut cn.streetAddress);
            Self::str_to_cert_cstr(&self.postalCode, &mut cn.postalCode);
            Self::str_to_cert_cstr(&self.email, &mut cn.email);
            Self::str_to_cert_cstr(&self.url, &mut cn.url);
            Self::str_to_cert_cstr(&self.host, &mut cn.host);
            return cn;
        }
    }
}

implement_to_from_json!(CertificateName);

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize)]
pub struct CertificateNetwork {
    pub id: NetworkId,
    pub controller: Fingerprint,
}

impl CertificateNetwork {
    pub(crate) fn new_from_capi(cn: &ztcore::ZT_Certificate_Network) -> CertificateNetwork {
        CertificateNetwork {
            id: NetworkId(cn.id),
            controller: Fingerprint {
                address: Address(cn.controller.address),
                hash: cn.controller.hash,
            },
        }
    }

    pub(crate) fn to_capi(&self) -> ztcore::ZT_Certificate_Network {
        ztcore::ZT_Certificate_Network {
            id: self.id.0,
            controller: ztcore::ZT_Fingerprint {
                address: self.controller.address.0,
                hash: self.controller.hash,
            },
        }
    }
}

implement_to_from_json!(CertificateNetwork);

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Serialize, Deserialize)]
pub struct CertificateIdentity {
    pub identity: Identity,
    pub locator: Option<Locator>,
}

impl CertificateIdentity {
    pub(crate) unsafe fn new_from_capi(ci: &ztcore::ZT_Certificate_Identity) -> Option<CertificateIdentity> {
        if ci.identity.is_null() {
            return None;
        }
        Some(CertificateIdentity {
            identity: Identity::new_from_capi(ci.identity, false).clone(),
            locator: if ci.locator.is_null() { None } else { Some(Locator::new_from_capi(ci.locator, false).clone()) },
        })
    }

    pub(crate) unsafe fn to_capi(&self) -> ztcore::ZT_Certificate_Identity {
        ztcore::ZT_Certificate_Identity {
            identity: self.identity.capi,
            locator: if self.locator.is_some() { self.locator.as_ref().unwrap().capi } else { null() },
        }
    }
}

implement_to_from_json!(CertificateIdentity);

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct CertificateSubject {
    pub timestamp: i64,
    pub identities: Vec<CertificateIdentity>,
    pub networks: Vec<CertificateNetwork>,
    pub certificates: Vec<CertificateSerialNo>,
    pub updateURLs: Vec<String>,
    pub name: CertificateName,
    pub uniqueId: Vec<u8>,
    pub uniqueIdProofSignature: Vec<u8>,
}

pub(crate) struct CertificateSubjectCAPIContainer {
    pub(crate) subject: ztcore::ZT_Certificate_Subject,
    subject_identities: Vec<ztcore::ZT_Certificate_Identity>,
    subject_networks: Vec<ztcore::ZT_Certificate_Network>,
    subject_certificates: Vec<*const u8>,
    subject_urls: Vec<*const c_char>,
    subject_urls_strs: Vec<CString>,
}

impl CertificateSubject {
    pub(crate) unsafe fn new_from_capi(cs: &ztcore::ZT_Certificate_Subject) -> CertificateSubject {
        unsafe {
            let mut identities: Vec<CertificateIdentity> = Vec::new();
            if !cs.identities.is_null() && cs.identityCount > 0 {
                let cidentities: &[ztcore::ZT_Certificate_Identity] = std::slice::from_raw_parts(cs.identities, cs.identityCount as usize);
                for i in cidentities.iter() {
                    let ci = CertificateIdentity::new_from_capi(i);
                    if ci.is_some() {
                        identities.push(ci.unwrap());
                    }
                }
            }

            let mut networks: Vec<CertificateNetwork> = Vec::new();
            if !cs.networks.is_null() && cs.networkCount > 0 {
                let cnetworks: &[ztcore::ZT_Certificate_Network] = std::slice::from_raw_parts(cs.networks, cs.networkCount as usize);
                for i in cnetworks.iter() {
                    networks.push(CertificateNetwork::new_from_capi(i));
                }
            }

            let mut certificates: Vec<CertificateSerialNo> = Vec::new();
            if !cs.certificates.is_null() && cs.certificateCount > 0 {
                let ccertificates: &[*const u8] = std::slice::from_raw_parts(cs.certificates, cs.certificateCount as usize);
                let mut ctmp: [u8; 48] = [0; 48];
                for i in ccertificates.iter() {
                    copy_nonoverlapping(*i, ctmp.as_mut_ptr(), 48);
                    certificates.push(CertificateSerialNo(ctmp));
                }
            }

            let mut update_urls: Vec<String> = Vec::new();
            if !cs.updateURLs.is_null() && cs.updateURLCount > 0 {
                let cupdate_urls: &[*const c_char] = std::slice::from_raw_parts(cs.updateURLs, cs.updateURLCount as usize);
                for i in cupdate_urls.iter() {
                    update_urls.push(cstr_to_string(*i, CERTIFICATE_MAX_STRING_LENGTH - 1));
                }
            }

            return CertificateSubject {
                timestamp: cs.timestamp,
                identities: identities,
                networks: networks,
                certificates: certificates,
                updateURLs: update_urls,
                name: CertificateName::new_from_capi(&cs.name),
                uniqueId: Vec::from(std::slice::from_raw_parts(cs.uniqueId, cs.uniqueIdSize as usize)),
                uniqueIdProofSignature: Vec::from(std::slice::from_raw_parts(cs.uniqueIdProofSignature, cs.uniqueIdProofSignatureSize as usize)),
            };
        }
    }

    pub(crate) unsafe fn to_capi(&self) -> CertificateSubjectCAPIContainer {
        let mut capi_identities: Vec<ztcore::ZT_Certificate_Identity> = Vec::new();
        let mut capi_networks: Vec<ztcore::ZT_Certificate_Network> = Vec::new();
        let mut capi_certificates: Vec<*const u8> = Vec::new();
        let mut capi_urls: Vec<*const c_char> = Vec::new();
        let mut capi_urls_strs: Vec<CString> = Vec::new();

        if !self.identities.is_empty() {
            capi_identities.reserve(self.identities.len());
            for i in self.identities.iter() {
                capi_identities.push(unsafe { (*i).to_capi() });
            }
        }
        if !self.networks.is_empty() {
            capi_networks.reserve(self.networks.len());
            for i in self.networks.iter() {
                capi_networks.push((*i).to_capi());
            }
        }
        if !self.certificates.is_empty() {
            capi_certificates.reserve(self.certificates.len());
            for i in self.certificates.iter() {
                capi_certificates.push((*i).0.as_ptr());
            }
        }
        if !self.updateURLs.is_empty() {
            capi_urls.reserve(self.updateURLs.len());
            for i in self.updateURLs.iter() {
                let cs = CString::new((*i).as_str());
                if cs.is_ok() {
                    capi_urls_strs.push(cs.unwrap());
                    capi_urls.push(capi_urls_strs.last().unwrap().as_ptr());
                }
            }
        }

        CertificateSubjectCAPIContainer {
            subject: ztcore::ZT_Certificate_Subject {
                timestamp: self.timestamp,
                identities: capi_identities.as_mut_ptr(),
                networks: capi_networks.as_mut_ptr(),
                certificates: capi_certificates.as_ptr(),
                updateURLs: capi_urls.as_ptr(),
                identityCount: capi_identities.len() as c_uint,
                networkCount: capi_networks.len() as c_uint,
                certificateCount: capi_certificates.len() as c_uint,
                updateURLCount: capi_urls.len() as c_uint,
                name: unsafe { self.name.to_capi() },
                uniqueId: self.uniqueId.as_ptr(),
                uniqueIdProofSignature: self.uniqueIdProofSignature.as_ptr(),
                uniqueIdSize: self.uniqueId.len() as c_uint,
                uniqueIdProofSignatureSize: self.uniqueIdProofSignature.len() as c_uint,
            },
            subject_identities: capi_identities,
            subject_networks: capi_networks,
            subject_certificates: capi_certificates,
            subject_urls: capi_urls,
            subject_urls_strs: capi_urls_strs,
        }
    }

    pub fn new_csr(&self, uid: Option<&CertificateSubjectUniqueIdSecret>) -> Result<Box<[u8]>, ResultCode> {
        let mut csr: Vec<u8> = Vec::new();
        csr.resize(16384, 0);
        let mut csr_size: c_int = 16384;

        unsafe {
            let capi = self.to_capi();
            if uid.is_some() {
                let uid2 = uid.unwrap();
                if ztcore::ZT_Certificate_newCSR(&capi.subject as *const ztcore::ZT_Certificate_Subject, uid2.public.as_ptr() as *const c_void, uid2.public.len() as c_int, uid2.private.as_ptr() as *const c_void, uid2.private.len() as c_int, csr.as_mut_ptr() as *mut c_void, &mut csr_size) != 0 {
                    return Err(ResultCode::ErrorBadParameter);
                }
            } else {
                if ztcore::ZT_Certificate_newCSR(&capi.subject, null(), -1, null(), -1, csr.as_mut_ptr() as *mut c_void, &mut csr_size) != 0 {
                    return Err(ResultCode::ErrorBadParameter);
                }
            }
        }

        return Ok(csr.into_boxed_slice());
    }
}

implement_to_from_json!(CertificateSubject);

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct Certificate {
    pub serialNo: CertificateSerialNo,
    pub flags: u64,
    pub timestamp: i64,
    pub validity: [i64; 2],
    pub subject: CertificateSubject,
    pub issuer: Identity,
    pub issuerName: CertificateName,
    pub extendedAttributes: Vec<u8>,
    pub maxPathLength: u32,
    pub crl: Vec<CertificateSerialNo>,
    pub signature: Vec<u8>,
}

pub(crate) struct CertificateCAPIContainer {
    pub(crate) certificate: ztcore::ZT_Certificate,
    certificate_crls: Vec<*const u8>,
    subject_container: CertificateSubjectCAPIContainer,
}

impl Certificate {
    pub(crate) unsafe fn new_from_capi(c: &ztcore::ZT_Certificate) -> Certificate {
        unsafe {
            let mut crl: Vec<CertificateSerialNo> = Vec::new();
            if !c.crl.is_null() && c.crlCount > 0 {
                let ccrl: &[*const u8] = std::slice::from_raw_parts(c.crl, c.crlCount as usize);
                let mut ctmp: [u8; 48] = [0; 48];
                for i in ccrl.iter() {
                    if !(*i).is_null() {
                        copy_nonoverlapping(*i, ctmp.as_mut_ptr(), 48);
                        crl.push(CertificateSerialNo(ctmp));
                    }
                }
            }

            return Certificate {
                serialNo: CertificateSerialNo(c.serialNo),
                flags: c.flags,
                timestamp: c.timestamp,
                validity: c.validity,
                subject: CertificateSubject::new_from_capi(&c.subject),
                issuer: Identity::new_from_capi(c.issuer, false),
                issuerName: CertificateName::new_from_capi(&c.issuerName),
                extendedAttributes: Vec::from(std::slice::from_raw_parts(c.extendedAttributes, c.extendedAttributesSize as usize)),
                maxPathLength: c.maxPathLength as u32,
                crl: crl,
                signature: Vec::from(std::slice::from_raw_parts(c.signature, c.signatureSize as usize)),
            };
        }
    }

    pub(crate) unsafe fn to_capi(&self) -> CertificateCAPIContainer {
        let mut capi_crls: Vec<*const u8> = Vec::new();
        capi_crls.reserve(self.crl.len());
        for i in self.crl.iter() {
            capi_crls.push((*i).0.as_ptr());
        }

        let subject = unsafe { self.subject.to_capi() };
        CertificateCAPIContainer {
            certificate: ztcore::ZT_Certificate {
                serialNo: self.serialNo.0,
                flags: self.flags,
                timestamp: self.timestamp,
                validity: self.validity,
                subject: subject.subject,
                issuer: self.issuer.capi,
                issuerName: unsafe { self.issuerName.to_capi() },
                extendedAttributes: self.extendedAttributes.as_ptr(),
                extendedAttributesSize: self.extendedAttributes.len() as c_uint,
                maxPathLength: self.maxPathLength as c_uint,
                crl: capi_crls.as_ptr(),
                crlCount: capi_crls.len() as c_uint,
                signature: self.signature.as_ptr(),
                signatureSize: self.signature.len() as c_uint,
            },
            certificate_crls: capi_crls,
            subject_container: subject,
        }
    }

    pub fn new_from_bytes(b: &[u8], verify: bool) -> Result<Certificate, CertificateError> {
        let mut capi_cert: *const ztcore::ZT_Certificate = null_mut();
        let capi_verify: c_int = if verify { 1 } else { 0 };
        let result = unsafe { ztcore::ZT_Certificate_decode(&mut capi_cert as *mut *const ztcore::ZT_Certificate, b.as_ptr() as *const c_void, b.len() as c_int, capi_verify) };
        if result != ztcore::ZT_CertificateError_ZT_CERTIFICATE_ERROR_NONE {
            return Err(CertificateError::from_u32(result as u32).unwrap_or(CertificateError::InvalidFormat));
        }
        if capi_cert.is_null() {
            return Err(CertificateError::InvalidFormat);
        }
        unsafe {
            let cert = Certificate::new_from_capi(&*capi_cert);
            ztcore::ZT_Certificate_delete(capi_cert);
            return Ok(cert);
        }
    }

    pub fn to_bytes(&self) -> Result<Box<[u8]>, ResultCode> {
        let mut cert: Vec<u8> = Vec::new();
        cert.resize(16384, 0);
        let mut cert_size: c_int = 16384;
        unsafe {
            let capi = self.to_capi();
            if ztcore::ZT_Certificate_encode(&capi.certificate as *const ztcore::ZT_Certificate, cert.as_mut_ptr() as *mut c_void, &mut cert_size) != 0 {
                return Err(ResultCode::ErrorInternalNonFatal);
            }
        }
        cert.resize(cert_size as usize, 0);
        return Ok(cert.into_boxed_slice());
    }

    pub fn sign(&self, id: &Identity) -> Result<Vec<u8>, ResultCode> {
        if !id.has_private() {
            return Err(ResultCode::ErrorBadParameter);
        }
        let mut signed_cert: Vec<u8> = Vec::new();
        signed_cert.resize(16384, 0);
        let mut signed_cert_size: c_int = 16384;
        unsafe {
            let capi = self.to_capi();
            if ztcore::ZT_Certificate_sign(&capi.certificate as *const ztcore::ZT_Certificate, id.capi, signed_cert.as_mut_ptr() as *mut c_void, &mut signed_cert_size) != 0 {
                return Err(ResultCode::ErrorBadParameter);
            }
        }
        signed_cert.resize(signed_cert_size as usize, 0);
        return Ok(signed_cert);
    }

    pub fn verify(&self) -> CertificateError {
        unsafe {
            let capi = self.to_capi();
            return CertificateError::from_u32(ztcore::ZT_Certificate_verify(&capi.certificate as *const ztcore::ZT_Certificate) as u32).unwrap_or(CertificateError::InvalidFormat);
        }
    }
}

implement_to_from_json!(Certificate);

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
