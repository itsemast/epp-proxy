//! EPP commands relating to domain objects

use std::convert::{TryFrom, TryInto};

use super::super::domain::{
    CheckRequest, CheckResponse, ClaimsCheckRequest, ClaimsCheckResponse, CreateData,
    CreateRequest, CreateResponse, DeleteRequest, DeleteResponse, InfoContact, InfoHost,
    InfoNameserver, InfoRequest, InfoResponse, PanData, RenewData, RenewRequest, RenewResponse,
    SecDNSDSData, SecDNSData, SecDNSDataType, SecDNSKeyData, Status, TrademarkCheckRequest,
    TransferAcceptRejectRequest, TransferData, TransferQueryRequest, TransferRequestRequest,
    TransferResponse, UpdateObject, UpdateRequest, UpdateResponse, UpdateSecDNSRemove,
    VerisignSyncRequest,
};
use super::super::{fee, launch, proto, Error, Period, PeriodUnit, Response};
use super::router::HandleReqReturn;
use super::ServerFeatures;

impl From<proto::domain::EPPDomainStatusType> for Status {
    fn from(from: proto::domain::EPPDomainStatusType) -> Self {
        use proto::domain::EPPDomainStatusType;
        match from {
            EPPDomainStatusType::ClientDeleteProhibited => Status::ClientDeleteProhibited,
            EPPDomainStatusType::ClientHold => Status::ClientHold,
            EPPDomainStatusType::ClientRenewProhibited => Status::ClientRenewProhibited,
            EPPDomainStatusType::ClientTransferProhibited => Status::ClientTransferProhibited,
            EPPDomainStatusType::ClientUpdateProhibited => Status::ClientUpdateProhibited,
            EPPDomainStatusType::Inactive => Status::Inactive,
            EPPDomainStatusType::Ok => Status::Ok,
            EPPDomainStatusType::Granted => Status::Ok,
            EPPDomainStatusType::PendingCreate => Status::PendingCreate,
            EPPDomainStatusType::PendingDelete => Status::PendingDelete,
            EPPDomainStatusType::Terminated => Status::PendingDelete,
            EPPDomainStatusType::PendingRenew => Status::PendingRenew,
            EPPDomainStatusType::PendingTransfer => Status::PendingTransfer,
            EPPDomainStatusType::PendingUpdate => Status::PendingUpdate,
            EPPDomainStatusType::ServerDeleteProhibited => Status::ServerDeleteProhibited,
            EPPDomainStatusType::ServerHold => Status::ServerHold,
            EPPDomainStatusType::ServerRenewProhibited => Status::ServerRenewProhibited,
            EPPDomainStatusType::ServerTransferProhibited => Status::ServerTransferProhibited,
            EPPDomainStatusType::ServerUpdateProhibited => Status::ServerUpdateProhibited,
        }
    }
}

impl From<&Status> for proto::domain::EPPDomainStatusType {
    fn from(from: &Status) -> Self {
        use proto::domain::EPPDomainStatusType;
        match from {
            Status::ClientDeleteProhibited => EPPDomainStatusType::ClientDeleteProhibited,
            Status::ClientHold => EPPDomainStatusType::ClientHold,
            Status::ClientRenewProhibited => EPPDomainStatusType::ClientRenewProhibited,
            Status::ClientTransferProhibited => EPPDomainStatusType::ClientTransferProhibited,
            Status::ClientUpdateProhibited => EPPDomainStatusType::ClientUpdateProhibited,
            Status::Inactive => EPPDomainStatusType::Inactive,
            Status::Ok => EPPDomainStatusType::Ok,
            Status::PendingCreate => EPPDomainStatusType::PendingCreate,
            Status::PendingDelete => EPPDomainStatusType::PendingDelete,
            Status::PendingRenew => EPPDomainStatusType::PendingRenew,
            Status::PendingTransfer => EPPDomainStatusType::PendingTransfer,
            Status::PendingUpdate => EPPDomainStatusType::PendingUpdate,
            Status::ServerDeleteProhibited => EPPDomainStatusType::ServerDeleteProhibited,
            Status::ServerHold => EPPDomainStatusType::ServerHold,
            Status::ServerRenewProhibited => EPPDomainStatusType::ServerRenewProhibited,
            Status::ServerTransferProhibited => EPPDomainStatusType::ServerTransferProhibited,
            Status::ServerUpdateProhibited => EPPDomainStatusType::ServerUpdateProhibited,
        }
    }
}

impl From<&InfoNameserver> for proto::domain::EPPDomainInfoNameserver {
    fn from(from: &InfoNameserver) -> Self {
        match from {
            InfoNameserver::HostOnly(h) => {
                proto::domain::EPPDomainInfoNameserver::HostOnly(h.to_string())
            }
            InfoNameserver::HostAndAddress {
                host, addresses, ..
            } => proto::domain::EPPDomainInfoNameserver::HostAndAddress {
                host: host.to_string(),
                addresses: addresses
                    .iter()
                    .map(|addr| proto::domain::EPPDomainInfoNameserverAddress {
                        address: addr.address.to_string(),
                        ip_version: match addr.ip_version {
                            super::super::host::AddressVersion::IPv4 => {
                                proto::domain::EPPDomainInfoNameserverAddressVersion::IPv4
                            }
                            super::super::host::AddressVersion::IPv6 => {
                                proto::domain::EPPDomainInfoNameserverAddressVersion::IPv6
                            }
                        },
                    })
                    .collect(),
            },
        }
    }
}

impl From<&Period> for proto::domain::EPPDomainPeriod {
    fn from(from: &Period) -> Self {
        proto::domain::EPPDomainPeriod {
            unit: match from.unit {
                PeriodUnit::Months => proto::domain::EPPDomainPeriodUnit::Months,
                PeriodUnit::Years => proto::domain::EPPDomainPeriodUnit::Years,
            },
            value: std::cmp::min(99, std::cmp::max(from.value, 1)),
        }
    }
}

impl From<&proto::domain::EPPDomainPeriod> for Period {
    fn from(from: &proto::domain::EPPDomainPeriod) -> Self {
        Period {
            unit: match from.unit {
                proto::domain::EPPDomainPeriodUnit::Years => PeriodUnit::Years,
                proto::domain::EPPDomainPeriodUnit::Months => PeriodUnit::Months,
            },
            value: from.value,
        }
    }
}

impl
    TryFrom<(
        proto::domain::EPPDomainInfoData,
        &Option<proto::EPPResponseExtension>,
    )> for InfoResponse
{
    type Error = Error;

    fn try_from(
        from: (
            proto::domain::EPPDomainInfoData,
            &Option<proto::EPPResponseExtension>,
        ),
    ) -> Result<Self, Self::Error> {
        let (domain_info, extension) = from;
        let rgp_state = match extension {
            Some(ext) => match ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPRGPInfo(i) => Some(i),
                _ => None,
            }) {
                Some(e) => e.state.iter().map(|s| (&s.state).into()).collect(),
                None => vec![],
            },
            None => vec![],
        };

        let sec_dns = match extension {
            Some(ext) => match ext
                .value
                .iter()
                .find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPSecDNSInfo(i) => Some(i),
                    _ => None,
                })
                .map(|i| {
                    Ok(SecDNSData {
                        max_sig_life: i.max_signature_life,
                        data: if !i.ds_data.is_empty() {
                            SecDNSDataType::DSData(
                                i.ds_data
                                    .iter()
                                    .map(|d| SecDNSDSData {
                                        key_tag: d.key_tag,
                                        algorithm: d.algorithm,
                                        digest_type: d.digest_type,
                                        digest: d.digest.clone(),
                                        key_data: d.key_data.as_ref().map(|k| SecDNSKeyData {
                                            flags: k.flags,
                                            protocol: k.protocol,
                                            algorithm: k.algorithm,
                                            public_key: k.public_key.clone(),
                                        }),
                                    })
                                    .collect(),
                            )
                        } else if !i.key_data.is_empty() {
                            SecDNSDataType::KeyData(
                                i.key_data
                                    .iter()
                                    .map(|k| SecDNSKeyData {
                                        flags: k.flags,
                                        protocol: k.protocol,
                                        algorithm: k.algorithm,
                                        public_key: k.public_key.clone(),
                                    })
                                    .collect(),
                            )
                        } else {
                            return Err(Error::ServerInternal);
                        },
                    })
                }) {
                Some(i) => match i {
                    Ok(i) => Some(i),
                    Err(e) => return Err(e),
                },
                None => None,
            },
            None => None,
        };

        let launch_info = match extension {
            Some(ext) => ext
                .value
                .iter()
                .find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPLaunchInfoData(i) => Some(i),
                    _ => None,
                })
                .map(launch::LaunchInfoData::from),
            None => None,
        };

        let donuts_fee_data = match extension {
            Some(ext) => {
                let charge = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPDonutsChargeInfoData(i) => Some(i),
                    _ => None,
                });

                charge.map(Into::into)
            }
            None => None,
        };

        let whois_info = match extension {
            Some(ext) => {
                let i = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::VerisignWhoisInfo(i) => Some(i),
                    _ => None,
                });
                i.map(Into::into)
            }
            None => None,
        };

        let isnic_info = match extension {
            Some(ext) => {
                let i = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::ISNICDomainInfo(i) => Some(i),
                    _ => None,
                });
                i.map(Into::into)
            }
            None => None,
        };

        let personal_registration = match extension {
            Some(ext) => {
                let i = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::PersonalRegistrationInfoData(i) => Some(i),
                    _ => None,
                });
                i.map(Into::into)
            }
            None => None,
        };

        let keysys = match extension {
            Some(ext) => {
                let i = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::KeysysResultData(
                        proto::keysys::ResultData::Domain(i),
                    ) => Some(i),
                    _ => None,
                });
                match i.map(TryInto::try_into) {
                    Some(Ok(i)) => Some(i),
                    Some(Err(e)) => return Err(e),
                    None => None,
                }
            }
            None => None,
        };

        let nominet_ext = match extension {
            Some(ext) => ext
                .value
                .iter()
                .find_map(|p| match p {
                    proto::EPPResponseExtensionType::NominetDomainExtInfo(i) => Some(i),
                    _ => None,
                })
                .map(Into::into),
            None => None,
        };

        Ok(InfoResponse {
            eurid_idn: super::eurid::extract_eurid_idn_singular(extension, domain_info.name.as_str())?,
            name: domain_info.name,
            registry_id: domain_info.registry_id.unwrap_or_default(),
            statuses: domain_info
                .statuses
                .into_iter()
                .map(|s| s.status.into())
                .collect(),
            registrant: domain_info.registrant.unwrap_or_default(),
            contacts: domain_info
                .contacts
                .into_iter()
                .map(|c| InfoContact {
                    contact_id: c.contact_id,
                    contact_type: c.contact_type,
                })
                .collect(),
            nameservers: match domain_info.nameservers {
                None => vec![],
                Some(n) => n
                    .servers
                    .into_iter()
                    .map(|s| Ok(match s {
                        proto::domain::EPPDomainInfoNameserver::HostOnly(h) => {
                            InfoNameserver::HostOnly(h)
                        }
                        proto::domain::EPPDomainInfoNameserver::HostAndAddress {
                            host,
                            addresses,
                        } => InfoNameserver::HostAndAddress {
                            eurid_idn: super::eurid::extract_eurid_idn_singular(extension, host.as_str())?,
                            host,
                            addresses: addresses
                                .into_iter()
                                .map(|addr| {
                                    super::super::host::Address {
                                        address: addr.address,
                                        ip_version: match addr.ip_version {
                                            proto::domain::EPPDomainInfoNameserverAddressVersion::IPv4 => {
                                                super::super::host::AddressVersion::IPv4
                                            }
                                            proto::domain::EPPDomainInfoNameserverAddressVersion::IPv6 => {
                                                super::super::host::AddressVersion::IPv6
                                            }
                                        },
                                    }
                                })
                                .collect(),
                        },
                    }))
                    .collect::<Result<Vec<_>, _>>()?,
            },
            hosts: domain_info.hosts,
            client_id: domain_info.client_id,
            client_created_id: domain_info.client_created_id,
            creation_date: domain_info.creation_date,
            expiry_date: domain_info.expiry_date,
            last_updated_client: domain_info.last_updated_client,
            last_updated_date: domain_info.last_updated_date,
            last_transfer_date: domain_info.last_transfer_date,
            rgp_state,
            auth_info: match domain_info.auth_info {
                Some(a) => a.password,
                None => None,
            },
            sec_dns,
            launch_info,
            donuts_fee_data,
            whois_info,
            isnic_info,
            eurid_data: super::eurid::extract_eurid_domain_info(extension),
            personal_registration,
            keysys,
            nominet_ext,
        })
    }
}

impl
    TryFrom<(
        proto::domain::EPPDomainTransferData,
        &Option<proto::EPPResponseExtension>,
    )> for TransferResponse
{
    type Error = Error;

    fn try_from(
        from: (
            proto::domain::EPPDomainTransferData,
            &Option<proto::EPPResponseExtension>,
        ),
    ) -> Result<Self, Self::Error> {
        let (domain_transfer, extension) = from;

        let fee_data = match extension {
            Some(ext) => {
                let fee10 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee10TransferData(i) => Some(i),
                    _ => None,
                });
                let fee011 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee011TransferData(i) => Some(i),
                    _ => None,
                });
                let fee09 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee09TransferData(i) => Some(i),
                    _ => None,
                });
                let fee08 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee08TransferData(i) => Some(i),
                    _ => None,
                });
                let fee07 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee07TransferData(i) => Some(i),
                    _ => None,
                });
                let fee06 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee06TransferData(i) => Some(i),
                    _ => None,
                });
                let fee05 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee05TransferData(i) => Some(i),
                    _ => None,
                });

                if let Some(f) = fee10 {
                    Some(f.into())
                } else if let Some(f) = fee011 {
                    Some(f.into())
                } else if let Some(f) = fee09 {
                    Some(f.into())
                } else if let Some(f) = fee08 {
                    Some(f.into())
                } else if let Some(f) = fee07 {
                    Some(f.into())
                } else if let Some(f) = fee06 {
                    Some(f.into())
                } else {
                    fee05.map(|f| f.into())
                }
            }
            None => None,
        };

        let donuts_fee_data = match extension {
            Some(ext) => {
                let charge = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPDonutsChargeTransferData(i) => Some(i),
                    _ => None,
                });

                charge.map(Into::into)
            }
            None => None,
        };

        let personal_registration = match extension {
            Some(ext) => {
                let i = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::PersonalRegistrationTransferData(i) => Some(i),
                    _ => None,
                });
                i.map(Into::into)
            }
            None => None,
        };

        Ok(TransferResponse {
            pending: false,
            data: TransferData {
                name: domain_transfer.name.clone(),
                status: (&domain_transfer.transfer_status).into(),
                requested_client_id: domain_transfer.requested_client_id.clone(),
                requested_date: domain_transfer.requested_date,
                act_client_id: domain_transfer.act_client_id.clone(),
                act_date: domain_transfer.act_date,
                expiry_date: domain_transfer.expiry_date,
                eurid_data: super::eurid::extract_eurid_domain_transfer_info(extension),
                eurid_idn: super::eurid::extract_eurid_idn_singular(extension, None)?,
                personal_registration,
            },
            fee_data,
            donuts_fee_data,
        })
    }
}

impl
    TryFrom<(
        Option<proto::domain::EPPDomainCreateData>,
        &Option<proto::EPPResponseExtension>,
    )> for CreateResponse
{
    type Error = Error;

    fn try_from(
        from: (
            Option<proto::domain::EPPDomainCreateData>,
            &Option<proto::EPPResponseExtension>,
        ),
    ) -> Result<Self, Self::Error> {
        let (domain_create, extension) = from;

        let fee_data = match extension {
            Some(ext) => {
                let fee10 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee10CreateData(i) => Some(i),
                    _ => None,
                });
                let fee011 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee011CreateData(i) => Some(i),
                    _ => None,
                });
                let fee09 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee09CreateData(i) => Some(i),
                    _ => None,
                });
                let fee08 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee08CreateData(i) => Some(i),
                    _ => None,
                });
                let fee07 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee07CreateData(i) => Some(i),
                    _ => None,
                });
                let fee06 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee06CreateData(i) => Some(i),
                    _ => None,
                });
                let fee05 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee05CreateData(i) => Some(i),
                    _ => None,
                });

                if let Some(f) = fee10 {
                    Some(f.into())
                } else if let Some(f) = fee011 {
                    Some(f.into())
                } else if let Some(f) = fee09 {
                    Some(f.into())
                } else if let Some(f) = fee08 {
                    Some(f.into())
                } else if let Some(f) = fee07 {
                    Some(f.into())
                } else if let Some(f) = fee06 {
                    Some(f.into())
                } else {
                    fee05.map(|f| f.into())
                }
            }
            None => None,
        };

        let launch_create = match extension {
            Some(ext) => ext
                .value
                .iter()
                .find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPLaunchCreateData(i) => Some(i),
                    _ => None,
                })
                .map(launch::LaunchCreateData::from),
            None => None,
        };

        let donuts_fee_data = match extension {
            Some(ext) => {
                let charge = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPDonutsChargeCreateData(i) => Some(i),
                    _ => None,
                });

                charge.map(Into::into)
            }
            None => None,
        };

        let personal_registration = match extension {
            Some(ext) => {
                let i = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::PersonalRegistrationCreateData(i) => Some(i),
                    _ => None,
                });
                i.map(Into::into)
            }
            None => None,
        };

        match domain_create {
            Some(domain_create) => Ok(CreateResponse {
                pending: false,
                data: CreateData {
                    eurid_idn: super::eurid::extract_eurid_idn_singular(
                        extension,
                        domain_create.name.as_str(),
                    )?,
                    name: domain_create.name.clone(),
                    creation_date: Some(domain_create.creation_date),
                    expiration_date: domain_create.expiry_date,
                    personal_registration,
                },
                fee_data,
                donuts_fee_data,
                launch_create,
            }),
            None => Ok(CreateResponse {
                pending: false,
                data: CreateData {
                    eurid_idn: super::eurid::extract_eurid_idn_singular(extension, None)?,
                    name: "".to_string(),
                    creation_date: None,
                    expiration_date: None,
                    personal_registration,
                },
                fee_data,
                donuts_fee_data,
                launch_create,
            }),
        }
    }
}

impl
    TryFrom<(
        proto::domain::EPPDomainRenewData,
        &Option<proto::EPPResponseExtension>,
    )> for RenewResponse
{
    type Error = Error;

    fn try_from(
        from: (
            proto::domain::EPPDomainRenewData,
            &Option<proto::EPPResponseExtension>,
        ),
    ) -> Result<Self, Self::Error> {
        let (domain_renew, extension) = from;

        let fee_data = match extension {
            Some(ext) => {
                let fee10 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee10RenewData(i) => Some(i),
                    _ => None,
                });
                let fee011 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee011RenewData(i) => Some(i),
                    _ => None,
                });
                let fee09 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee09RenewData(i) => Some(i),
                    _ => None,
                });
                let fee08 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee08RenewData(i) => Some(i),
                    _ => None,
                });
                let fee07 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee07RenewData(i) => Some(i),
                    _ => None,
                });
                let fee06 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee06RenewData(i) => Some(i),
                    _ => None,
                });
                let fee05 = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPFee05RenewData(i) => Some(i),
                    _ => None,
                });

                if let Some(f) = fee10 {
                    Some(f.into())
                } else if let Some(f) = fee011 {
                    Some(f.into())
                } else if let Some(f) = fee09 {
                    Some(f.into())
                } else if let Some(f) = fee08 {
                    Some(f.into())
                } else if let Some(f) = fee07 {
                    Some(f.into())
                } else if let Some(f) = fee06 {
                    Some(f.into())
                } else {
                    fee05.map(|f| f.into())
                }
            }
            None => None,
        };

        let donuts_fee_data = match extension {
            Some(ext) => {
                let charge = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::EPPDonutsChargeRenewData(i) => Some(i),
                    _ => None,
                });

                charge.map(Into::into)
            }
            None => None,
        };

        let personal_registration = match extension {
            Some(ext) => {
                let i = ext.value.iter().find_map(|p| match p {
                    proto::EPPResponseExtensionType::PersonalRegistrationRenewData(i) => Some(i),
                    _ => None,
                });
                i.map(Into::into)
            }
            None => None,
        };

        Ok(RenewResponse {
            pending: false,
            data: RenewData {
                eurid_idn: super::eurid::extract_eurid_idn_singular(
                    extension,
                    domain_renew.name.as_str(),
                )?,
                name: domain_renew.name.to_owned(),
                new_expiry_date: domain_renew.expiry_date,
                eurid_data: super::eurid::extract_eurid_domain_renew_info(extension),
                personal_registration,
            },
            fee_data,
            donuts_fee_data,
        })
    }
}

impl From<&proto::domain::EPPDomainPanData> for PanData {
    fn from(from: &proto::domain::EPPDomainPanData) -> Self {
        PanData {
            name: from.name.domain.clone(),
            result: from.name.result,
            server_transaction_id: from.transaction_id.server_transaction_id.clone(),
            client_transaction_id: from.transaction_id.client_transaction_id.clone(),
            date: from.action_date,
        }
    }
}

pub(crate) fn check_domain<T>(id: &str) -> Result<(), Response<T>> {
    if !id.is_empty() {
        Ok(())
    } else {
        Err(Err(Error::Err(
            "domain name has a min length of 1".to_string(),
        )))
    }
}

pub(crate) fn check_pass<T>(id: &str) -> Result<(), Response<T>> {
    if id.len() > 6 {
        Ok(())
    } else {
        Err(Err(Error::Err(
            "passwords have a min length of 6".to_string(),
        )))
    }
}

pub fn handle_check(client: &ServerFeatures, req: &CheckRequest) -> HandleReqReturn<CheckResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;
    let command = proto::EPPCheck::Domain(proto::domain::EPPDomainCheck {
        name: req.name.clone(),
        auth_info: None,
    });
    let mut ext = vec![];

    super::verisign::handle_verisign_namestore_erratum(client, &mut ext);

    if let Some(fee_check) = &req.fee_check {
        if client.fee_supported {
            ext.push(proto::EPPCommandExtensionType::EPPFee10Check(
                proto::fee::EPPFee10Check {
                    currency: fee_check.currency.to_owned(),
                    commands: fee_check
                        .commands
                        .iter()
                        .map(|c| {
                            Ok(proto::fee::EPPFee10CheckCommand {
                                name: match (&c.command).into() {
                                    Some(n) => n,
                                    None => return Err(Err(Error::Unsupported)),
                                },
                                phase: c.phase.as_ref().map(Into::into),
                                subphase: c.sub_phase.as_ref().map(Into::into),
                                period: c.period.as_ref().map(Into::into),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                },
            ))
        } else if client.fee_011_supported {
            fee_check
                .commands
                .iter()
                .map(|c| {
                    ext.push(proto::EPPCommandExtensionType::EPPFee011Check(
                        proto::fee::EPPFee011Check {
                            currency: fee_check.currency.to_owned(),
                            command: proto::fee::EPPFeeCommand {
                                command: match (&c.command).into() {
                                    Some(n) => n,
                                    None => return Err(Err(Error::Unsupported)),
                                },
                                phase: c.phase.as_ref().map(Into::into),
                                subphase: c.sub_phase.as_ref().map(Into::into),
                            },
                            period: c.period.as_ref().map(Into::into),
                        },
                    ));
                    Ok(())
                })
                .collect::<Result<Vec<_>, _>>()?;
        } else if client.fee_09_supported {
            ext.push(proto::EPPCommandExtensionType::EPPFee09Check(
                proto::fee::EPPFee09Check {
                    objects: fee_check
                        .commands
                        .iter()
                        .map(|c| {
                            Ok(proto::fee::EPPFee09CheckObject {
                                object_uri: Some("urn:ietf:params:xml:ns:domain-1.0".to_string()),
                                object_id: proto::fee::EPPFee10ObjectID {
                                    element: "name".to_string(),
                                    id: req.name.to_owned(),
                                },
                                currency: fee_check.currency.to_owned(),
                                command: proto::fee::EPPFeeCommand {
                                    command: match (&c.command).into() {
                                        Some(n) => n,
                                        None => return Err(Err(Error::Unsupported)),
                                    },
                                    phase: c.phase.as_ref().map(Into::into),
                                    subphase: c.sub_phase.as_ref().map(Into::into),
                                },
                                period: c.period.as_ref().map(Into::into),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                },
            ))
        } else if client.fee_08_supported {
            ext.push(proto::EPPCommandExtensionType::EPPFee08Check(
                proto::fee::EPPFee08Check {
                    domains: fee_check
                        .commands
                        .iter()
                        .map(|c| {
                            Ok(proto::fee::EPPFee08CheckDomain {
                                name: req.name.to_owned(),
                                currency: fee_check.currency.to_owned(),
                                command: proto::fee::EPPFeeCommand {
                                    command: match (&c.command).into() {
                                        Some(n) => n,
                                        None => return Err(Err(Error::Unsupported)),
                                    },
                                    phase: c.phase.as_ref().map(Into::into),
                                    subphase: c.sub_phase.as_ref().map(Into::into),
                                },
                                period: c.period.as_ref().map(Into::into),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                },
            ))
        } else if client.fee_07_supported {
            ext.push(proto::EPPCommandExtensionType::EPPFee07Check(
                proto::fee::EPPFee07Check {
                    domains: fee_check
                        .commands
                        .iter()
                        .map(|c| {
                            Ok(proto::fee::EPPFee07CheckDomain {
                                name: req.name.to_owned(),
                                currency: fee_check.currency.to_owned(),
                                command: proto::fee::EPPFeeCommand {
                                    command: match (&c.command).into() {
                                        Some(n) => n,
                                        None => return Err(Err(Error::Unsupported)),
                                    },
                                    phase: c.phase.as_ref().map(Into::into),
                                    subphase: c.sub_phase.as_ref().map(Into::into),
                                },
                                period: c.period.as_ref().map(Into::into),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                },
            ))
        } else if client.fee_06_supported {
            ext.push(proto::EPPCommandExtensionType::EPPFee06Check(
                proto::fee::EPPFee06Check {
                    domains: fee_check
                        .commands
                        .iter()
                        .map(|c| {
                            Ok(proto::fee::EPPFee06CheckDomain {
                                name: req.name.to_owned(),
                                currency: fee_check.currency.to_owned(),
                                command: proto::fee::EPPFeeCommand {
                                    command: match (&c.command).into() {
                                        Some(n) => n,
                                        None => return Err(Err(Error::Unsupported)),
                                    },
                                    phase: c.phase.as_ref().map(Into::into),
                                    subphase: c.sub_phase.as_ref().map(Into::into),
                                },
                                period: c.period.as_ref().map(Into::into),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                },
            ))
        } else if client.fee_05_supported {
            ext.push(proto::EPPCommandExtensionType::EPPFee05Check(
                proto::fee::EPPFee05Check {
                    domains: fee_check
                        .commands
                        .iter()
                        .map(|c| {
                            Ok(proto::fee::EPPFee05CheckDomain {
                                name: req.name.to_owned(),
                                currency: fee_check.currency.to_owned(),
                                command: proto::fee::EPPFeeCommand {
                                    command: match (&c.command).into() {
                                        Some(n) => n,
                                        None => return Err(Err(Error::Unsupported)),
                                    },
                                    phase: c.phase.as_ref().map(Into::into),
                                    subphase: c.sub_phase.as_ref().map(Into::into),
                                },
                                period: c.period.as_ref().map(Into::into),
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                },
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(launch_check) = &req.launch_check {
        if client.launch_supported {
            ext.push(proto::EPPCommandExtensionType::EPPLaunchCheck(
                launch_check.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(keysys) = &req.keysys {
        if client.keysys_supported {
            ext.push(proto::EPPCommandExtensionType::KeysysCheck(
                proto::keysys::Check::Domain(proto::keysys::DomainCheck {
                    allocation_token: keysys.allocation_token.to_owned(),
                }),
            ));
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    Ok((
        proto::EPPCommandType::Check(command),
        match ext.is_empty() {
            true => None,
            false => Some(ext),
        },
    ))
}

pub fn handle_check_response(
    response: proto::EPPResponse, _metrics: &crate::metrics::ScopedMetrics
) -> Response<CheckResponse> {
    let fee_check = match &response.extension {
        Some(ext) => {
            let fee10 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee10CheckData(i) => Some(i),
                _ => None,
            });
            let fee011 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee011CheckData(i) => Some(i),
                _ => None,
            });
            let fee09 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee09CheckData(i) => Some(i),
                _ => None,
            });
            let fee08 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee08CheckData(i) => Some(i),
                _ => None,
            });
            let fee07 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee07CheckData(i) => Some(i),
                _ => None,
            });
            let fee06 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee06CheckData(i) => Some(i),
                _ => None,
            });
            let fee05 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee05CheckData(i) => Some(i),
                _ => None,
            });

            if let Some(f) = fee10 {
                let d = match f.objects.get(0) {
                    Some(o) => o,
                    None => return Err(Error::ServerInternal),
                };
                Some(fee::FeeCheckData {
                    available: d.available,
                    commands: d
                        .commands
                        .iter()
                        .map(|c| fee::FeeCommand {
                            command: (&c.name).into(),
                            period: c.period.as_ref().map(Into::into),
                            standard: Some(c.standard),
                            currency: f.currency.to_owned(),
                            fees: c.fee.iter().map(Into::into).collect(),
                            credits: c.credit.iter().map(Into::into).collect(),
                            reason: c.reason.to_owned(),
                            class: d.class.to_owned(),
                        })
                        .collect(),
                    reason: d.reason.to_owned(),
                })
            } else if let Some(f) = fee011 {
                let d = match f.objects.get(0) {
                    Some(o) => o,
                    None => return Err(Error::ServerInternal),
                };
                Some(fee::FeeCheckData {
                    available: d.available,
                    commands: f
                        .objects
                        .iter()
                        .map(|c| fee::FeeCommand {
                            command: (&c.command.name.command).into(),
                            period: c.period.as_ref().map(Into::into),
                            standard: Some(c.command.standard),
                            currency: c.currency.to_owned(),
                            fees: c.fee.iter().map(Into::into).collect(),
                            credits: c.credit.iter().map(Into::into).collect(),
                            reason: c.reason.to_owned(),
                            class: c.class.to_owned(),
                        })
                        .collect(),
                    reason: d.reason.to_owned(),
                })
            } else if let Some(f) = fee09 {
                Some(fee::FeeCheckData {
                    available: true,
                    commands: f
                        .objects
                        .iter()
                        .map(|d| fee::FeeCommand {
                            command: (&d.command.command).into(),
                            period: d.period.as_ref().map(Into::into),
                            standard: None,
                            currency: d.currency.to_owned(),
                            fees: d.fee.iter().map(Into::into).collect(),
                            credits: d.credit.iter().map(Into::into).collect(),
                            class: d.class.to_owned(),
                            reason: None,
                        })
                        .collect(),
                    reason: None,
                })
            } else if let Some(f) = fee08 {
                Some(fee::FeeCheckData {
                    available: true,
                    commands: f
                        .domains
                        .iter()
                        .map(|d| fee::FeeCommand {
                            command: (&d.command.command).into(),
                            period: d.period.as_ref().map(Into::into),
                            standard: None,
                            currency: d.currency.to_owned(),
                            fees: d.fee.iter().map(Into::into).collect(),
                            credits: d.credit.iter().map(Into::into).collect(),
                            class: d.class.to_owned(),
                            reason: None,
                        })
                        .collect(),
                    reason: None,
                })
            } else if let Some(f) = fee07 {
                Some(fee::FeeCheckData {
                    available: true,
                    commands: f
                        .domains
                        .iter()
                        .map(|d| fee::FeeCommand {
                            command: (&d.command.command).into(),
                            period: d.period.as_ref().map(Into::into),
                            standard: None,
                            currency: d.currency.to_owned().unwrap_or_default(),
                            fees: d.fee.iter().map(Into::into).collect(),
                            credits: d.credit.iter().map(Into::into).collect(),
                            class: d.class.to_owned(),
                            reason: None,
                        })
                        .collect(),
                    reason: None,
                })
            } else if let Some(f) = fee06 {
                Some(fee::FeeCheckData {
                    available: true,
                    commands: f
                        .domains
                        .iter()
                        .map(|d| fee::FeeCommand {
                            command: (&d.command.command).into(),
                            period: Some((&d.period).into()),
                            standard: None,
                            currency: d.currency.to_owned(),
                            fees: d.fee.iter().map(Into::into).collect(),
                            class: d.class.to_owned(),
                            credits: vec![],
                            reason: None,
                        })
                        .collect(),
                    reason: None,
                })
            } else {
                fee05.map(|f| fee::FeeCheckData {
                    available: true,
                    commands: f
                        .domains
                        .iter()
                        .map(|d| fee::FeeCommand {
                            command: (&d.command.command).into(),
                            period: Some((&d.period).into()),
                            standard: None,
                            currency: d.currency.to_owned(),
                            fees: d.fee.iter().map(Into::into).collect(),
                            class: d.class.to_owned(),
                            credits: vec![],
                            reason: None,
                        })
                        .collect(),
                    reason: None,
                })
            }
        }
        None => None,
    };

    let donuts_fee_check = match &response.extension {
        Some(ext) => {
            let charge = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPDonutsChargeCheckData(i) => Some(i),
                _ => None,
            });

            if let Some(c) = charge {
                let d = match c.domains.get(0) {
                    Some(o) => o,
                    None => return Err(Error::ServerInternal),
                };
                Some(d.into())
            } else {
                None
            }
        }
        None => None,
    };

    match response.data {
        Some(value) => match value.value {
            proto::EPPResultDataValue::EPPDomainCheckResult(domain_check) => {
                if let Some(domain_check) = domain_check.data.first() {
                    Response::Ok(CheckResponse {
                        eurid_idn: super::eurid::extract_eurid_idn_singular(
                            &response.extension,
                            domain_check.name.name.as_str(),
                        )?,
                        avail: domain_check.name.available,
                        reason: domain_check.reason.to_owned(),
                        fee_check,
                        donuts_fee_check,
                        eurid_check: super::eurid::extract_eurid_domain_check_singular(
                            &response.extension,
                        )?,
                    })
                } else {
                    Err(Error::ServerInternal)
                }
            }
            _ => Err(Error::ServerInternal),
        },
        None => Err(Error::ServerInternal),
    }
}

pub fn handle_claims_check(
    client: &ServerFeatures,
    req: &ClaimsCheckRequest,
) -> HandleReqReturn<ClaimsCheckResponse> {
    if !client.launch_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;
    let command = proto::EPPCheck::Domain(proto::domain::EPPDomainCheck {
        name: req.name.clone(),
        auth_info: None,
    });
    let mut ext = vec![proto::EPPCommandExtensionType::EPPLaunchCheck(
        (&req.launch_check).into(),
    )];

    super::verisign::handle_verisign_namestore_erratum(client, &mut ext);
    Ok((
        proto::EPPCommandType::Check(command),
        match ext.is_empty() {
            true => None,
            false => Some(ext),
        },
    ))
}

pub fn handle_trademark_check(
    client: &ServerFeatures,
    req: &TrademarkCheckRequest,
) -> HandleReqReturn<ClaimsCheckResponse> {
    if !client.launch_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;
    let command = proto::EPPCheck::Domain(proto::domain::EPPDomainCheck {
        name: req.name.clone(),
        auth_info: None,
    });
    let mut ext = vec![proto::EPPCommandExtensionType::EPPLaunchCheck(
        (&launch::LaunchTrademarkCheck {}).into(),
    )];

    super::verisign::handle_verisign_namestore_erratum(client, &mut ext);
    Ok((
        proto::EPPCommandType::Check(command),
        match ext.is_empty() {
            true => None,
            false => Some(ext),
        },
    ))
}

pub fn handle_claims_check_response(
    response: proto::EPPResponse, _metrics: &crate::metrics::ScopedMetrics
) -> Response<ClaimsCheckResponse> {
    let claims_check = match response.extension {
        Some(ext) => {
            let claims = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPLaunchCheckData(i) => Some(i),
                _ => None,
            });

            if let Some(c) = claims {
                c.data.first().map(|domain_check| ClaimsCheckResponse {
                    exists: domain_check.name.exists,
                    claims_key: domain_check.claim_key.iter().map(Into::into).collect(),
                })
            } else {
                None
            }
        }
        None => None,
    };

    match response.data {
        Some(_) => Err(Error::ServerInternal),
        None => match claims_check {
            Some(c) => Response::Ok(c),
            None => Err(Error::ServerInternal),
        },
    }
}

pub fn handle_info(client: &ServerFeatures, req: &InfoRequest) -> HandleReqReturn<InfoResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;
    let command = proto::EPPInfo::Domain(proto::domain::EPPDomainInfo {
        name: proto::domain::EPPDomainInfoName {
            name: req.name.clone(),
            hosts: req.hosts.as_ref().map(|h| match h {
                InfoHost::All => proto::domain::EPPDomainInfoHosts::All,
                InfoHost::Delegated => proto::domain::EPPDomainInfoHosts::Delegated,
                InfoHost::Subordinate => proto::domain::EPPDomainInfoHosts::Subordinate,
                InfoHost::None => proto::domain::EPPDomainInfoHosts::None,
            }),
        },
        auth_info: req
            .auth_info
            .as_ref()
            .map(|a| proto::domain::EPPDomainAuthInfo {
                password: Some(a.clone()),
            }),
    });
    let mut exts = vec![];
    if let Some(launch_info) = &req.launch_info {
        if client.launch_supported {
            exts.push(proto::EPPCommandExtensionType::EPPLaunchInfo(
                launch_info.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }
    if client.verisign_whois_info {
        exts.push(proto::EPPCommandExtensionType::VerisignWhoisInfExt(
            proto::verisign::EPPWhoisInfoExt { flag: true },
        ))
    }

    if let Some(eurid_data) = &req.eurid_data {
        if let Some(euird_auth_info) = eurid_data.into() {
            if client.eurid_auth_info_supported {
                exts.push(proto::EPPCommandExtensionType::EURIDAuthInfo(
                    euird_auth_info,
                ))
            } else {
                return Err(Err(Error::Unsupported));
            }
        }
    }

    super::verisign::handle_verisign_namestore_erratum(client, &mut exts);

    Ok((
        proto::EPPCommandType::Info(command),
        match exts.is_empty() {
            true => None,
            false => Some(exts),
        },
    ))
}

pub fn handle_info_response(
    response: proto::EPPResponse, _metrics: &crate::metrics::ScopedMetrics
) -> Response<InfoResponse> {
    match response.data {
        Some(value) => match value.value {
            proto::EPPResultDataValue::EPPDomainInfoResult(domain_info) => {
                (*domain_info, &response.extension).try_into()
            }
            _ => Err(Error::ServerInternal),
        },
        None => Err(Error::ServerInternal),
    }
}

pub fn handle_create(
    client: &ServerFeatures,
    req: &CreateRequest,
) -> HandleReqReturn<CreateResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;
    let no_registrant = client.has_erratum("verisign-com")
        || client.has_erratum("verisign-net")
        || client.has_erratum("verisign-cc")
        || client.has_erratum("verisign-tv");
    if !no_registrant {
        super::contact::check_id(&req.registrant)?;
    }

    let mut exts = vec![];
    match &req.sec_dns {
        Some(sec_dns) => {
            if client.secdns_supported || client.has_erratum("pir") {
                exts.push(proto::EPPCommandExtensionType::EPPSecDNSCreate(
                    match &sec_dns.data {
                        SecDNSDataType::DSData(ds_data) => proto::secdns::EPPSecDNSData {
                            max_signature_life: sec_dns.max_sig_life,
                            key_data: vec![],
                            ds_data: ds_data
                                .iter()
                                .map(|d| proto::secdns::EPPSecDNSDSData {
                                    key_tag: d.key_tag,
                                    algorithm: d.algorithm,
                                    digest_type: d.digest_type,
                                    digest: d.digest.clone(),
                                    key_data: d.key_data.as_ref().map(|k| {
                                        proto::secdns::EPPSecDNSKeyData {
                                            flags: k.flags,
                                            protocol: k.protocol,
                                            algorithm: k.algorithm,
                                            public_key: k.public_key.clone(),
                                        }
                                    }),
                                })
                                .collect(),
                        },
                        SecDNSDataType::KeyData(key_data) => proto::secdns::EPPSecDNSData {
                            max_signature_life: sec_dns.max_sig_life,
                            ds_data: vec![],
                            key_data: key_data
                                .iter()
                                .map(|k| proto::secdns::EPPSecDNSKeyData {
                                    flags: k.flags,
                                    protocol: k.protocol,
                                    algorithm: k.algorithm,
                                    public_key: k.public_key.clone(),
                                })
                                .collect(),
                        },
                    },
                ))
            } else {
                return Err(Err(Error::Unsupported));
            }
        }
        None => {}
    }

    if let Some(launch_create) = &req.launch_create {
        if client.launch_supported {
            if !launch_create.core_nic.is_empty() {
                if !(client.corenic_mark || client.has_erratum("corenic")) {
                    return Err(Err(Error::Unsupported));
                }

                for info in launch_create.core_nic.iter() {
                    if let Some(info_type) = &info.info_type {
                        if info_type.is_empty() || info_type.len() > 64 {
                            return Err(Err(Error::Err(
                                "application info type has a min length of 1 and a max length of 64".to_string(),
                            )));
                        }
                    }
                    if info.info.is_empty() || info.info.len() > 2048 {
                        return Err(Err(Error::Err(
                            "application info has a min length of 1 and a max length of 2048"
                                .to_string(),
                        )));
                    }
                }
            }

            exts.push(proto::EPPCommandExtensionType::EPPLaunchCreate(
                launch_create.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(fee_agreement) = &req.fee_agreement {
        if client.fee_supported {
            exts.push(proto::EPPCommandExtensionType::EPPFee10Create(
                fee_agreement.into(),
            ));
        } else if client.fee_011_supported {
            exts.push(proto::EPPCommandExtensionType::EPPFee011Create(
                fee_agreement.into(),
            ));
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(eurid_data) = &req.eurid_data {
        if client.eurid_domain_support {
            exts.push(proto::EPPCommandExtensionType::EURIDDomainCreate(
                eurid_data.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    match &req.isnic_payment {
        Some(e) => {
            if client.isnic_domain_supported {
                exts.push(proto::EPPCommandExtensionType::ISNICDomainCreate(e.into()))
            } else {
                return Err(Err(Error::Unsupported));
            }
        }
        None => {
            if client.isnic_domain_supported {
                return Err(Err(Error::Err(
                    "payment extension required for ISNIC".to_string(),
                )));
            }
        }
    }

    if let Some(personal_registration_data) = &req.personal_registration {
        if client.personal_registration_supported {
            exts.push(proto::EPPCommandExtensionType::PersonalRegistrationCreate(
                personal_registration_data.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(keysys) = &req.keysys {
        if client.keysys_supported {
            let mut e = proto::keysys::DomainCreate {
                accept_premium_price: Some(keysys.accept_premium_price),
                accept_ssl_requirement: Some(keysys.accept_ssl_requirements),
                allocation_token: keysys.allocation_token.as_ref().cloned(),
                ca_legal_type: None,
                ca_trademark: None,
                eu_accept_trustee_tac: None,
                eu_registrant_lang: None,
                eu_registrant_citizenship: None,
                de_abuse_contact: None,
                de_accept_trustee_tac: None,
                de_general_request: None,
                de_holder_person: None,
                fr_accept_trustee_tac: None,
                gay_accept_requirements: None,
                intended_use: None,
                name_emailforward: None,
                rs_owner_idcard: None,
                rs_owner_company_number: None,
                rs_admin_idcard: None,
                rs_admin_company_number: None,
                rs_tech_idcard: None,
                rs_tech_company_number: None,
                us_purpose: None,
                us_category: None,
                us_validator: None,
                renewal_mode: Some(match keysys.renewal_mode {
                    super::super::keysys::RenewalMode::Default => {
                        proto::keysys::RenewalMode::Default
                    }
                    super::super::keysys::RenewalMode::AutoDelete => {
                        proto::keysys::RenewalMode::AutoDelete
                    }
                    super::super::keysys::RenewalMode::AutoExpire => {
                        proto::keysys::RenewalMode::AutoExpire
                    }
                    super::super::keysys::RenewalMode::AutoRenew => {
                        proto::keysys::RenewalMode::AutoRenew
                    }
                    super::super::keysys::RenewalMode::AutoRenewQuarterly => {
                        proto::keysys::RenewalMode::AutoRenewQuarterly
                    }
                    super::super::keysys::RenewalMode::AutoRenewMonthly => {
                        proto::keysys::RenewalMode::AutoRenewMonthly
                    }
                    super::super::keysys::RenewalMode::ExpireAuction => {
                        proto::keysys::RenewalMode::ExpireAuction
                    }
                    super::super::keysys::RenewalMode::RenewOnce => {
                        proto::keysys::RenewalMode::RenewOnce
                    }
                }),
                transfer_mode: Some(match keysys.transfer_mode {
                    super::super::keysys::TransferMode::Default => {
                        proto::keysys::TransferMode::Default
                    }
                    super::super::keysys::TransferMode::AutoApprove => {
                        proto::keysys::TransferMode::AutoApprove
                    }
                    super::super::keysys::TransferMode::AutoDeny => {
                        proto::keysys::TransferMode::AutoDeny
                    }
                }),
                whois_banner_0: keysys.whois_banner.get(0).cloned(),
                whois_banner_1: keysys.whois_banner.get(1).cloned(),
                whois_rsp: keysys.whois_rsp.as_ref().cloned(),
                whois_url: keysys.whois_url.as_ref().cloned(),
            };

            match &keysys.tld {
                None => {}
                Some(super::super::keysys::DomainCreateTLD::CA(d)) => {
                    e.ca_legal_type = Some(match d.legal_type {
                        super::super::keysys::CALegalType::CanadianPoliticalParty => proto::keysys::CALegalType::CanadianPoliticalParty,
                        super::super::keysys::CALegalType::AboriginalPeoples => proto::keysys::CALegalType::AboriginalPeoples,
                        super::super::keysys::CALegalType::CanadianUnincorporatedAssociation => proto::keysys::CALegalType::CanadianUnincorporatedAssociation,
                        super::super::keysys::CALegalType::Corporation => proto::keysys::CALegalType::Corporation,
                        super::super::keysys::CALegalType::Citizen => proto::keysys::CALegalType::Citizen,
                        super::super::keysys::CALegalType::CanadianEducationalInstitution => proto::keysys::CALegalType::CanadianEducationalInstitution,
                        super::super::keysys::CALegalType::Government => proto::keysys::CALegalType::Government,
                        super::super::keysys::CALegalType::CanadianHospital => proto::keysys::CALegalType::CanadianHospital,
                        super::super::keysys::CALegalType::IndianBand => proto::keysys::CALegalType::IndianBand,
                        super::super::keysys::CALegalType::CanadianLibraryArchiveMuseum => proto::keysys::CALegalType::CanadianLibraryArchiveMuseum,
                        super::super::keysys::CALegalType::LegalRepOfCanadianCitizenOrPermanentResident => proto::keysys::CALegalType::LegalRepOfCanadianCitizenOrPermanentResident,
                        super::super::keysys::CALegalType::TheQueen => proto::keysys::CALegalType::TheQueen,
                        super::super::keysys::CALegalType::OfficialMark => proto::keysys::CALegalType::OfficialMark,
                        super::super::keysys::CALegalType::Partnership => proto::keysys::CALegalType::Partnership,
                        super::super::keysys::CALegalType::PermanentResident => proto::keysys::CALegalType::PermanentResident,
                        super::super::keysys::CALegalType::TradeMark => proto::keysys::CALegalType::TradeMark,
                        super::super::keysys::CALegalType::TradeUnion => proto::keysys::CALegalType::TradeUnion,
                        super::super::keysys::CALegalType::Trust => proto::keysys::CALegalType::Trust,
                    });
                    e.ca_trademark = Some(d.trademark);
                }
                Some(super::super::keysys::DomainCreateTLD::EU(d)) => {
                    e.eu_accept_trustee_tac = Some(d.accept_trustee_tac);
                    e.eu_registrant_lang = d.registrant_lang.as_ref().map(|c| match c {
                        super::super::keysys::EULanguage::Bulgarian => {
                            proto::keysys::EULanguage::Bulgarian
                        }
                        super::super::keysys::EULanguage::Czech => proto::keysys::EULanguage::Czech,
                        super::super::keysys::EULanguage::Croatian => {
                            proto::keysys::EULanguage::Croatian
                        }
                        super::super::keysys::EULanguage::DutchFlemish => {
                            proto::keysys::EULanguage::DutchFlemish
                        }
                        super::super::keysys::EULanguage::Danish => {
                            proto::keysys::EULanguage::Danish
                        }
                        super::super::keysys::EULanguage::Estonian => {
                            proto::keysys::EULanguage::Estonian
                        }
                        super::super::keysys::EULanguage::English => {
                            proto::keysys::EULanguage::English
                        }
                        super::super::keysys::EULanguage::French => {
                            proto::keysys::EULanguage::French
                        }
                        super::super::keysys::EULanguage::Finnish => {
                            proto::keysys::EULanguage::Finnish
                        }
                        super::super::keysys::EULanguage::Gaelic => {
                            proto::keysys::EULanguage::Gaelic
                        }
                        super::super::keysys::EULanguage::German => {
                            proto::keysys::EULanguage::German
                        }
                        super::super::keysys::EULanguage::Hungarian => {
                            proto::keysys::EULanguage::Hungarian
                        }
                        super::super::keysys::EULanguage::Italian => {
                            proto::keysys::EULanguage::Italian
                        }
                        super::super::keysys::EULanguage::Latvian => {
                            proto::keysys::EULanguage::Latvian
                        }
                        super::super::keysys::EULanguage::Lithuanian => {
                            proto::keysys::EULanguage::Lithuanian
                        }
                        super::super::keysys::EULanguage::Maltese => {
                            proto::keysys::EULanguage::Maltese
                        }
                        super::super::keysys::EULanguage::ModernGreek => {
                            proto::keysys::EULanguage::ModernGreek
                        }
                        super::super::keysys::EULanguage::Portuguese => {
                            proto::keysys::EULanguage::Portuguese
                        }
                        super::super::keysys::EULanguage::Polish => {
                            proto::keysys::EULanguage::Polish
                        }
                        super::super::keysys::EULanguage::Romanian => {
                            proto::keysys::EULanguage::Romanian
                        }
                        super::super::keysys::EULanguage::Swedish => {
                            proto::keysys::EULanguage::Swedish
                        }
                        super::super::keysys::EULanguage::Slovene => {
                            proto::keysys::EULanguage::Slovene
                        }
                        super::super::keysys::EULanguage::Slovak => {
                            proto::keysys::EULanguage::Slovak
                        }
                        super::super::keysys::EULanguage::Spanish => {
                            proto::keysys::EULanguage::Spanish
                        }
                    });
                    e.eu_registrant_citizenship =
                        d.registrant_citizenship.as_ref().map(|c| match c {
                            super::super::keysys::EUCountry::Austria => {
                                proto::keysys::EUCountry::Austria
                            }
                            super::super::keysys::EUCountry::Bulgaria => {
                                proto::keysys::EUCountry::Bulgaria
                            }
                            super::super::keysys::EUCountry::Belgium => {
                                proto::keysys::EUCountry::Belgium
                            }
                            super::super::keysys::EUCountry::Croatia => {
                                proto::keysys::EUCountry::Croatia
                            }
                            super::super::keysys::EUCountry::Cyprus => {
                                proto::keysys::EUCountry::Cyprus
                            }
                            super::super::keysys::EUCountry::Czech => {
                                proto::keysys::EUCountry::Czech
                            }
                            super::super::keysys::EUCountry::Denmark => {
                                proto::keysys::EUCountry::Denmark
                            }
                            super::super::keysys::EUCountry::Estonia => {
                                proto::keysys::EUCountry::Estonia
                            }
                            super::super::keysys::EUCountry::France => {
                                proto::keysys::EUCountry::France
                            }
                            super::super::keysys::EUCountry::Finland => {
                                proto::keysys::EUCountry::Finland
                            }
                            super::super::keysys::EUCountry::Greece => {
                                proto::keysys::EUCountry::Greece
                            }
                            super::super::keysys::EUCountry::Germany => {
                                proto::keysys::EUCountry::Germany
                            }
                            super::super::keysys::EUCountry::Hungary => {
                                proto::keysys::EUCountry::Hungary
                            }
                            super::super::keysys::EUCountry::Italy => {
                                proto::keysys::EUCountry::Italy
                            }
                            super::super::keysys::EUCountry::Ireland => {
                                proto::keysys::EUCountry::Ireland
                            }
                            super::super::keysys::EUCountry::Latvia => {
                                proto::keysys::EUCountry::Latvia
                            }
                            super::super::keysys::EUCountry::Luxembourg => {
                                proto::keysys::EUCountry::Luxembourg
                            }
                            super::super::keysys::EUCountry::Lithuania => {
                                proto::keysys::EUCountry::Lithuania
                            }
                            super::super::keysys::EUCountry::Liechtenstein => {
                                proto::keysys::EUCountry::Liechtenstein
                            }
                            super::super::keysys::EUCountry::Malta => {
                                proto::keysys::EUCountry::Malta
                            }
                            super::super::keysys::EUCountry::Netherlands => {
                                proto::keysys::EUCountry::Netherlands
                            }
                            super::super::keysys::EUCountry::Portugal => {
                                proto::keysys::EUCountry::Portugal
                            }
                            super::super::keysys::EUCountry::Poland => {
                                proto::keysys::EUCountry::Poland
                            }
                            super::super::keysys::EUCountry::Romania => {
                                proto::keysys::EUCountry::Romania
                            }
                            super::super::keysys::EUCountry::Slovenia => {
                                proto::keysys::EUCountry::Slovenia
                            }
                            super::super::keysys::EUCountry::Slovakia => {
                                proto::keysys::EUCountry::Slovakia
                            }
                            super::super::keysys::EUCountry::Spain => {
                                proto::keysys::EUCountry::Spain
                            }
                            super::super::keysys::EUCountry::Sweden => {
                                proto::keysys::EUCountry::Sweden
                            }
                        });
                }
                Some(super::super::keysys::DomainCreateTLD::DE(d)) => {
                    e.de_abuse_contact = d.abuse_contact.as_ref().cloned();
                    e.de_accept_trustee_tac = Some(match d.accept_trustee_tac {
                        super::super::keysys::DETrustee::None => proto::keysys::DETrustee::None,
                        super::super::keysys::DETrustee::Annually => {
                            proto::keysys::DETrustee::Annual
                        }
                        super::super::keysys::DETrustee::Monthly => {
                            proto::keysys::DETrustee::Monthly
                        }
                    });
                    e.de_general_request = d.general_request.as_ref().cloned();
                    e.de_holder_person = Some(d.holder_person);
                }
                Some(super::super::keysys::DomainCreateTLD::FR(d)) => {
                    e.fr_accept_trustee_tac = Some(d.accept_trustee_tac);
                }
                Some(super::super::keysys::DomainCreateTLD::Gay(d)) => {
                    e.gay_accept_requirements = Some(d.accept_requirements);
                }
                Some(super::super::keysys::DomainCreateTLD::Name(d)) => {
                    e.name_emailforward = d.email_forward.as_ref().cloned();
                }
                Some(super::super::keysys::DomainCreateTLD::RS(d)) => {
                    match &d.owner {
                        super::super::keysys::RsId::IDCard(n) => {
                            e.rs_owner_idcard = Some(n.clone())
                        }
                        super::super::keysys::RsId::CompanyNumber(n) => {
                            e.rs_owner_company_number = Some(n.clone())
                        }
                    }
                    match &d.tech {
                        super::super::keysys::RsId::IDCard(n) => e.rs_tech_idcard = Some(n.clone()),
                        super::super::keysys::RsId::CompanyNumber(n) => {
                            e.rs_tech_company_number = Some(n.clone())
                        }
                    }
                    match &d.admin {
                        super::super::keysys::RsId::IDCard(n) => {
                            e.rs_admin_idcard = Some(n.clone())
                        }
                        super::super::keysys::RsId::CompanyNumber(n) => {
                            e.rs_admin_company_number = Some(n.clone())
                        }
                    }
                }
                Some(super::super::keysys::DomainCreateTLD::US(d)) => {
                    e.us_purpose = Some(match d.purpose {
                        super::super::keysys::USPurpose::Business => {
                            proto::keysys::USPurpose::Business
                        }
                        super::super::keysys::USPurpose::Government => {
                            proto::keysys::USPurpose::Government
                        }
                        super::super::keysys::USPurpose::Personal => {
                            proto::keysys::USPurpose::Personal
                        }
                        super::super::keysys::USPurpose::NonProfit => {
                            proto::keysys::USPurpose::NonProfit
                        }
                        super::super::keysys::USPurpose::Educational => {
                            proto::keysys::USPurpose::Educational
                        }
                    });
                    e.us_category = Some(match d.category {
                        super::super::keysys::USCategory::Citizen => {
                            proto::keysys::USCategory::Citizen
                        }
                        super::super::keysys::USCategory::PermanentResident => {
                            proto::keysys::USCategory::PermanentResident
                        }
                        super::super::keysys::USCategory::USOrganisation => {
                            proto::keysys::USCategory::USOrganisation
                        }
                        super::super::keysys::USCategory::OfficeOrFacility => {
                            proto::keysys::USCategory::OfficeOrFacility
                        }
                        super::super::keysys::USCategory::RegularActivity => {
                            proto::keysys::USCategory::RegularActivity
                        }
                    });
                    e.us_validator = d.validator.as_ref().cloned();
                }
            }

            exts.push(proto::EPPCommandExtensionType::KeysysCreate(
                proto::keysys::Create::Domain(e),
            ));
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(nominet_ext) = &req.nominet_ext {
        if client.nominet_domain_ext {
            exts.push(proto::EPPCommandExtensionType::NominetDomainExtCreate(
                nominet_ext.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    super::verisign::handle_verisign_namestore_erratum(client, &mut exts);
    super::fee::handle_donuts_fee_agreement(client, &req.donuts_fee_agreement, &mut exts)?;

    if !req.auth_info.is_empty() {
        check_pass(&req.auth_info)?;
    }

    let command = proto::EPPCreate::Domain(proto::domain::EPPDomainCreate {
        name: req.name.clone(),
        period: req.period.as_ref().map(|p| p.into()),
        nameservers: match req.nameservers.len() {
            0 => None,
            _ => Some(proto::domain::EPPDomainInfoNameservers {
                servers: req.nameservers.iter().map(|n| n.into()).collect(),
            }),
        },
        registrant: if no_registrant {
            None
        } else {
            Some(req.registrant.to_string())
        },
        contacts: req
            .contacts
            .iter()
            .map(|c| {
                super::contact::check_id(&c.contact_id)?;
                Ok(proto::domain::EPPDomainInfoContact {
                    contact_type: c.contact_type.to_string(),
                    contact_id: c.contact_id.to_string(),
                })
            })
            .collect::<Result<Vec<_>, super::router::Response<CreateResponse>>>()?,
        auth_info: proto::domain::EPPDomainAuthInfo {
            password: Some(req.auth_info.to_string()),
        },
    });

    Ok((
        proto::EPPCommandType::Create(command),
        match exts.len() {
            0 => None,
            _ => Some(exts),
        },
    ))
}

pub fn handle_create_response(
    response: proto::EPPResponse, _metrics: &crate::metrics::ScopedMetrics
) -> Response<CreateResponse> {
    let pending = response.is_pending();
    match response.data {
        Some(value) => match value.value {
            proto::EPPResultDataValue::EPPDomainCreateResult(domain_create) => {
                let mut res: CreateResponse =
                    (Some(domain_create), &response.extension).try_into()?;
                res.pending = pending;
                Ok(res)
            }
            _ => Err(Error::ServerInternal),
        },
        None => {
            if response.is_pending() {
                let mut res: CreateResponse = (None, &response.extension).try_into()?;
                res.pending = response.is_pending();
                Ok(res)
            } else {
                Err(Error::ServerInternal)
            }
        }
    }
}

pub fn handle_delete(
    client: &ServerFeatures,
    req: &DeleteRequest,
) -> HandleReqReturn<DeleteResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;
    let command = proto::EPPDelete::Domain(proto::domain::EPPDomainCheck {
        name: req.name.clone(),
        auth_info: None,
    });
    let mut ext = vec![];

    if let Some(eurid_data) = &req.eurid_data {
        if client.eurid_domain_support {
            ext.push(proto::EPPCommandExtensionType::EURIDDomainDelete(
                eurid_data.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(keysys) = &req.keysys {
        if client.keysys_supported {
            ext.push(proto::EPPCommandExtensionType::KeysysDelete(
                proto::keysys::Delete::Domain(proto::keysys::DomainDelete {
                    action: match keysys.action {
                        super::super::keysys::DomainDeleteAction::Default => None,
                        super::super::keysys::DomainDeleteAction::AutoExpire => {
                            Some(proto::keysys::DomainDeleteAction::AutoDelete)
                        }
                        super::super::keysys::DomainDeleteAction::AutoDelete => {
                            Some(proto::keysys::DomainDeleteAction::AutoDelete)
                        }
                        super::super::keysys::DomainDeleteAction::Instant => {
                            Some(proto::keysys::DomainDeleteAction::Instant)
                        }
                        super::super::keysys::DomainDeleteAction::Push => {
                            Some(proto::keysys::DomainDeleteAction::Push)
                        }
                    },
                    target: keysys.target.as_ref().cloned(),
                }),
            ));
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    super::verisign::handle_verisign_namestore_erratum(client, &mut ext);
    super::fee::handle_donuts_fee_agreement(client, &req.donuts_fee_agreement, &mut ext)?;

    if let Some(launch_info) = &req.launch_info {
        if client.launch_supported {
            ext.push(proto::EPPCommandExtensionType::EPPLaunchDelete(
                launch_info.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }
    Ok((
        proto::EPPCommandType::Delete(command),
        match ext.len() {
            0 => None,
            _ => Some(ext),
        },
    ))
}

pub fn handle_delete_response(
    response: proto::EPPResponse, _metrics: &crate::metrics::ScopedMetrics
) -> Response<DeleteResponse> {
    let fee_data = match &response.extension {
        Some(ext) => {
            let fee10 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee10DeleteData(i) => Some(i),
                _ => None,
            });
            let fee09 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee09DeleteData(i) => Some(i),
                _ => None,
            });
            let fee08 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee08DeleteData(i) => Some(i),
                _ => None,
            });
            let fee07 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee07DeleteData(i) => Some(i),
                _ => None,
            });
            let fee06 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee06DeleteData(i) => Some(i),
                _ => None,
            });
            let fee05 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee05DeleteData(i) => Some(i),
                _ => None,
            });

            if let Some(f) = fee10 {
                Some(f.into())
            } else if let Some(f) = fee09 {
                Some(f.into())
            } else if let Some(f) = fee08 {
                Some(f.into())
            } else if let Some(f) = fee07 {
                Some(f.into())
            } else if let Some(f) = fee06 {
                Some(f.into())
            } else {
                fee05.map(|f| f.into())
            }
        }
        None => None,
    };

    Response::Ok(DeleteResponse {
        pending: response.is_pending(),
        fee_data,
        eurid_idn: super::eurid::extract_eurid_idn_singular(&response.extension, None)?,
    })
}

pub fn handle_update(
    client: &ServerFeatures,
    req: &UpdateRequest,
) -> HandleReqReturn<UpdateResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;

    let no_registrant = client.has_erratum("verisign-com")
        || client.has_erratum("verisign-net")
        || client.has_erratum("verisign-cc")
        || client.has_erratum("verisign-tv");

    if !no_registrant {
        if let Some(new_registrant) = &req.new_registrant {
            super::contact::check_id(new_registrant)?;
        }
    }
    let mut adds = vec![];
    let mut rems = vec![];
    let mut add_ns = vec![];
    let mut rem_ns = vec![];
    for add in &req.add {
        match add {
            UpdateObject::Status(s) => adds.push(proto::domain::EPPDomainUpdateParam::Status(
                proto::domain::EPPDomainStatus {
                    status: s.into(),
                    message: None,
                },
            )),
            UpdateObject::Contact(c) => {
                super::contact::check_id(&c.contact_id)?;
                adds.push(proto::domain::EPPDomainUpdateParam::Contact(
                    proto::domain::EPPDomainInfoContact {
                        contact_type: c.contact_type.clone(),
                        contact_id: c.contact_id.clone(),
                    },
                ))
            }
            UpdateObject::Nameserver(n) => add_ns.push(n.into()),
        }
    }
    for rem in &req.remove {
        match rem {
            UpdateObject::Status(s) => rems.push(proto::domain::EPPDomainUpdateParam::Status(
                proto::domain::EPPDomainStatus {
                    status: s.into(),
                    message: None,
                },
            )),
            UpdateObject::Contact(c) => {
                super::contact::check_id(&c.contact_id)?;
                rems.push(proto::domain::EPPDomainUpdateParam::Contact(
                    proto::domain::EPPDomainInfoContact {
                        contact_type: c.contact_type.clone(),
                        contact_id: c.contact_id.clone(),
                    },
                ))
            }
            UpdateObject::Nameserver(n) => rem_ns.push(n.into()),
        }
    }
    if !add_ns.is_empty() {
        adds.push(proto::domain::EPPDomainUpdateParam::Nameserver(
            proto::domain::EPPDomainInfoNameservers { servers: add_ns },
        ))
    }
    if !rem_ns.is_empty() {
        rems.push(proto::domain::EPPDomainUpdateParam::Nameserver(
            proto::domain::EPPDomainInfoNameservers { servers: rem_ns },
        ))
    }

    let update_as_i32 = |u: &proto::domain::EPPDomainUpdateParam| match u {
        proto::domain::EPPDomainUpdateParam::Nameserver(_) => 0,
        proto::domain::EPPDomainUpdateParam::Contact(_) => 1,
        proto::domain::EPPDomainUpdateParam::Status(_) => 2,
    };
    adds.sort_unstable_by_key(update_as_i32);
    rems.sort_unstable_by_key(update_as_i32);

    let is_not_change = req.new_registrant.is_none() && req.new_auth_info.is_none();

    let is_not_isnic_change = match &req.isnic_info {
        Some(e) => !e.remove_all_ns && e.new_master_ns.is_empty(),
        None => true,
    };
    let is_not_eurid_change = match &req.eurid_data {
        Some(e) => {
            e.remove_on_site.is_none()
                && e.remove_reseller.is_none()
                && e.add_reseller.is_none()
                && e.add_on_site.is_none()
        }
        None => true,
    };
    let is_not_keysys_change = match &req.keysys {
        Some(e) => {
            e.whois_url.is_none()
                && e.whois_rsp.is_none()
                && e.whois_banner.is_empty()
                && e.tld.is_none()
                && e.renewal_mode.is_none()
                && e.transfer_mode.is_none()
        }
        None => true,
    };
    let is_not_nominet_change = match &req.nominet_ext {
        Some(e) => {
            e.first_bill.is_none()
                && e.recur_bill.is_none()
                && e.next_bill.is_none()
                && e.auto_bill.is_none()
                && e.next_period.is_none()
                && e.auto_period.is_none()
                && e.renew_not_required.is_none()
                && e.notes.is_empty()
                && e.reseller.is_none()
        }
        None => true,
    };

    if req.add.is_empty()
        && req.remove.is_empty()
        && is_not_change
        && (req.sec_dns.is_none() || !client.secdns_supported)
        && is_not_eurid_change
        && is_not_isnic_change
        && is_not_keysys_change
        && is_not_nominet_change
    {
        return Err(Err(Error::Err(
            "at least one operation must be specified".to_string(),
        )));
    }

    let mut exts = vec![];
    match &req.sec_dns {
        Some(sec_dns) => {
            if client.secdns_supported || client.has_erratum("pir") {
                exts.push(proto::EPPCommandExtensionType::EPPSecDNSUpdate(
                    proto::secdns::EPPSecDNSUpdate {
                        urgent: sec_dns.urgent,
                        add: sec_dns.add.as_ref().map(|a| match a {
                            SecDNSDataType::DSData(ds_data) => proto::secdns::EPPSecDNSUpdateAdd {
                                key_data: vec![],
                                ds_data: ds_data
                                    .iter()
                                    .map(|d| proto::secdns::EPPSecDNSDSData {
                                        key_tag: d.key_tag,
                                        algorithm: d.algorithm,
                                        digest_type: d.digest_type,
                                        digest: d.digest.clone(),
                                        key_data: d.key_data.as_ref().map(|k| {
                                            proto::secdns::EPPSecDNSKeyData {
                                                flags: k.flags,
                                                protocol: k.protocol,
                                                algorithm: k.algorithm,
                                                public_key: k.public_key.clone(),
                                            }
                                        }),
                                    })
                                    .collect(),
                            },
                            SecDNSDataType::KeyData(key_data) => {
                                proto::secdns::EPPSecDNSUpdateAdd {
                                    ds_data: vec![],
                                    key_data: key_data
                                        .iter()
                                        .map(|k| proto::secdns::EPPSecDNSKeyData {
                                            flags: k.flags,
                                            protocol: k.protocol,
                                            algorithm: k.algorithm,
                                            public_key: k.public_key.clone(),
                                        })
                                        .collect(),
                                }
                            }
                        }),
                        remove: sec_dns.remove.as_ref().map(|r| match r {
                            UpdateSecDNSRemove::All(a) => proto::secdns::EPPSecDNSUpdateRemove {
                                all: Some(*a),
                                ds_data: vec![],
                                key_data: vec![],
                            },
                            UpdateSecDNSRemove::Data(d) => match d {
                                SecDNSDataType::DSData(ds_data) => {
                                    proto::secdns::EPPSecDNSUpdateRemove {
                                        all: None,
                                        key_data: vec![],
                                        ds_data: ds_data
                                            .iter()
                                            .map(|d| proto::secdns::EPPSecDNSDSData {
                                                key_tag: d.key_tag,
                                                algorithm: d.algorithm,
                                                digest_type: d.digest_type,
                                                digest: d.digest.clone(),
                                                key_data: None,
                                            })
                                            .collect(),
                                    }
                                }
                                SecDNSDataType::KeyData(key_data) => {
                                    proto::secdns::EPPSecDNSUpdateRemove {
                                        all: None,
                                        ds_data: vec![],
                                        key_data: key_data
                                            .iter()
                                            .map(|k| proto::secdns::EPPSecDNSKeyData {
                                                flags: k.flags,
                                                protocol: k.protocol,
                                                algorithm: k.algorithm,
                                                public_key: k.public_key.clone(),
                                            })
                                            .collect(),
                                    }
                                }
                            },
                        }),
                        change: sec_dns.new_max_sig_life.map(|s| {
                            proto::secdns::EPPSecDNSUpdateChange {
                                max_signature_life: Some(s),
                            }
                        }),
                    },
                ))
            } else {
                return Err(Err(Error::Unsupported));
            }
        }
        None => {}
    }

    if let Some(fee_agreement) = &req.fee_agreement {
        if client.fee_supported {
            exts.push(proto::EPPCommandExtensionType::EPPFee10Update(
                fee_agreement.into(),
            ));
        } else if client.fee_011_supported {
            exts.push(proto::EPPCommandExtensionType::EPPFee011Update(
                fee_agreement.into(),
            ));
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    super::verisign::handle_verisign_namestore_erratum(client, &mut exts);
    super::fee::handle_donuts_fee_agreement(client, &req.donuts_fee_agreement, &mut exts)?;

    if let Some(launch_info) = &req.launch_info {
        if client.launch_supported {
            exts.push(proto::EPPCommandExtensionType::EPPLaunchUpdate(
                launch_info.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(eurid_data) = &req.eurid_data {
        if client.eurid_domain_support {
            exts.push(proto::EPPCommandExtensionType::EURIDDomainUpdate(
                eurid_data.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(isnic_info) = &req.isnic_info {
        if client.isnic_contact_supported {
            exts.push(proto::EPPCommandExtensionType::ISNICDomainUpdate(
                isnic_info.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(auth_info) = &req.new_auth_info {
        if !auth_info.is_empty() {
            check_pass(auth_info)?;
        }
    }

    if let Some(keysys) = &req.keysys {
        if client.keysys_supported {
            let mut e = proto::keysys::DomainUpdate {
                ca_legal_type: None,
                ca_trademark: None,
                eu_accept_trustee_tac: None,
                eu_registrant_lang: None,
                eu_registrant_citizenship: None,
                de_abuse_contact: None,
                de_accept_trustee_tac: None,
                de_general_request: None,
                de_holder_person: None,
                fr_accept_trustee_tac: None,
                name_emailforward: None,
                rs_owner_idcard: None,
                rs_owner_company_number: None,
                rs_admin_idcard: None,
                rs_admin_company_number: None,
                rs_tech_idcard: None,
                rs_tech_company_number: None,
                us_purpose: None,
                us_category: None,
                us_validator: None,
                renewal_mode: keysys.renewal_mode.as_ref().map(|m| match m {
                    super::super::keysys::RenewalMode::Default => {
                        proto::keysys::RenewalMode::Default
                    }
                    super::super::keysys::RenewalMode::AutoDelete => {
                        proto::keysys::RenewalMode::AutoDelete
                    }
                    super::super::keysys::RenewalMode::AutoExpire => {
                        proto::keysys::RenewalMode::AutoExpire
                    }
                    super::super::keysys::RenewalMode::AutoRenew => {
                        proto::keysys::RenewalMode::AutoRenew
                    }
                    super::super::keysys::RenewalMode::AutoRenewQuarterly => {
                        proto::keysys::RenewalMode::AutoRenewQuarterly
                    }
                    super::super::keysys::RenewalMode::AutoRenewMonthly => {
                        proto::keysys::RenewalMode::AutoRenewMonthly
                    }
                    super::super::keysys::RenewalMode::ExpireAuction => {
                        proto::keysys::RenewalMode::ExpireAuction
                    }
                    super::super::keysys::RenewalMode::RenewOnce => {
                        proto::keysys::RenewalMode::RenewOnce
                    }
                }),
                transfer_mode: keysys.transfer_mode.as_ref().map(|m| match m {
                    super::super::keysys::TransferMode::Default => {
                        proto::keysys::TransferMode::Default
                    }
                    super::super::keysys::TransferMode::AutoApprove => {
                        proto::keysys::TransferMode::AutoApprove
                    }
                    super::super::keysys::TransferMode::AutoDeny => {
                        proto::keysys::TransferMode::AutoDeny
                    }
                }),
                whois_banner_0: keysys.whois_banner.get(0).cloned(),
                whois_banner_1: keysys.whois_banner.get(1).cloned(),
                whois_rsp: keysys.whois_rsp.as_ref().cloned(),
                whois_url: keysys.whois_url.as_ref().cloned(),
            };

            match &keysys.tld {
                None => {}
                Some(super::super::keysys::DomainUpdateTLD::CA(d)) => {
                    e.ca_legal_type = d.legal_type.as_ref().map(|t| match t {
                        super::super::keysys::CALegalType::CanadianPoliticalParty => proto::keysys::CALegalType::CanadianPoliticalParty,
                        super::super::keysys::CALegalType::AboriginalPeoples => proto::keysys::CALegalType::AboriginalPeoples,
                        super::super::keysys::CALegalType::CanadianUnincorporatedAssociation => proto::keysys::CALegalType::CanadianUnincorporatedAssociation,
                        super::super::keysys::CALegalType::Corporation => proto::keysys::CALegalType::Corporation,
                        super::super::keysys::CALegalType::Citizen => proto::keysys::CALegalType::Citizen,
                        super::super::keysys::CALegalType::CanadianEducationalInstitution => proto::keysys::CALegalType::CanadianEducationalInstitution,
                        super::super::keysys::CALegalType::Government => proto::keysys::CALegalType::Government,
                        super::super::keysys::CALegalType::CanadianHospital => proto::keysys::CALegalType::CanadianHospital,
                        super::super::keysys::CALegalType::IndianBand => proto::keysys::CALegalType::IndianBand,
                        super::super::keysys::CALegalType::CanadianLibraryArchiveMuseum => proto::keysys::CALegalType::CanadianLibraryArchiveMuseum,
                        super::super::keysys::CALegalType::LegalRepOfCanadianCitizenOrPermanentResident => proto::keysys::CALegalType::LegalRepOfCanadianCitizenOrPermanentResident,
                        super::super::keysys::CALegalType::TheQueen => proto::keysys::CALegalType::TheQueen,
                        super::super::keysys::CALegalType::OfficialMark => proto::keysys::CALegalType::OfficialMark,
                        super::super::keysys::CALegalType::Partnership => proto::keysys::CALegalType::Partnership,
                        super::super::keysys::CALegalType::PermanentResident => proto::keysys::CALegalType::PermanentResident,
                        super::super::keysys::CALegalType::TradeMark => proto::keysys::CALegalType::TradeMark,
                        super::super::keysys::CALegalType::TradeUnion => proto::keysys::CALegalType::TradeUnion,
                        super::super::keysys::CALegalType::Trust => proto::keysys::CALegalType::Trust,
                    });
                    e.ca_trademark = d.trademark;
                }
                Some(super::super::keysys::DomainUpdateTLD::EU(d)) => {
                    e.eu_accept_trustee_tac = d.accept_trustee_tac;
                    e.eu_registrant_lang = d.registrant_lang.as_ref().map(|c| match c {
                        super::super::keysys::EULanguage::Bulgarian => {
                            proto::keysys::EULanguage::Bulgarian
                        }
                        super::super::keysys::EULanguage::Czech => proto::keysys::EULanguage::Czech,
                        super::super::keysys::EULanguage::Croatian => {
                            proto::keysys::EULanguage::Croatian
                        }
                        super::super::keysys::EULanguage::DutchFlemish => {
                            proto::keysys::EULanguage::DutchFlemish
                        }
                        super::super::keysys::EULanguage::Danish => {
                            proto::keysys::EULanguage::Danish
                        }
                        super::super::keysys::EULanguage::Estonian => {
                            proto::keysys::EULanguage::Estonian
                        }
                        super::super::keysys::EULanguage::English => {
                            proto::keysys::EULanguage::English
                        }
                        super::super::keysys::EULanguage::French => {
                            proto::keysys::EULanguage::French
                        }
                        super::super::keysys::EULanguage::Finnish => {
                            proto::keysys::EULanguage::Finnish
                        }
                        super::super::keysys::EULanguage::Gaelic => {
                            proto::keysys::EULanguage::Gaelic
                        }
                        super::super::keysys::EULanguage::German => {
                            proto::keysys::EULanguage::German
                        }
                        super::super::keysys::EULanguage::Hungarian => {
                            proto::keysys::EULanguage::Hungarian
                        }
                        super::super::keysys::EULanguage::Italian => {
                            proto::keysys::EULanguage::Italian
                        }
                        super::super::keysys::EULanguage::Latvian => {
                            proto::keysys::EULanguage::Latvian
                        }
                        super::super::keysys::EULanguage::Lithuanian => {
                            proto::keysys::EULanguage::Lithuanian
                        }
                        super::super::keysys::EULanguage::Maltese => {
                            proto::keysys::EULanguage::Maltese
                        }
                        super::super::keysys::EULanguage::ModernGreek => {
                            proto::keysys::EULanguage::ModernGreek
                        }
                        super::super::keysys::EULanguage::Portuguese => {
                            proto::keysys::EULanguage::Portuguese
                        }
                        super::super::keysys::EULanguage::Polish => {
                            proto::keysys::EULanguage::Polish
                        }
                        super::super::keysys::EULanguage::Romanian => {
                            proto::keysys::EULanguage::Romanian
                        }
                        super::super::keysys::EULanguage::Swedish => {
                            proto::keysys::EULanguage::Swedish
                        }
                        super::super::keysys::EULanguage::Slovene => {
                            proto::keysys::EULanguage::Slovene
                        }
                        super::super::keysys::EULanguage::Slovak => {
                            proto::keysys::EULanguage::Slovak
                        }
                        super::super::keysys::EULanguage::Spanish => {
                            proto::keysys::EULanguage::Spanish
                        }
                    });
                    e.eu_registrant_citizenship =
                        d.registrant_citizenship.as_ref().map(|c| match c {
                            super::super::keysys::EUCountry::Austria => {
                                proto::keysys::EUCountry::Austria
                            }
                            super::super::keysys::EUCountry::Bulgaria => {
                                proto::keysys::EUCountry::Bulgaria
                            }
                            super::super::keysys::EUCountry::Belgium => {
                                proto::keysys::EUCountry::Belgium
                            }
                            super::super::keysys::EUCountry::Croatia => {
                                proto::keysys::EUCountry::Croatia
                            }
                            super::super::keysys::EUCountry::Cyprus => {
                                proto::keysys::EUCountry::Cyprus
                            }
                            super::super::keysys::EUCountry::Czech => {
                                proto::keysys::EUCountry::Czech
                            }
                            super::super::keysys::EUCountry::Denmark => {
                                proto::keysys::EUCountry::Denmark
                            }
                            super::super::keysys::EUCountry::Estonia => {
                                proto::keysys::EUCountry::Estonia
                            }
                            super::super::keysys::EUCountry::France => {
                                proto::keysys::EUCountry::France
                            }
                            super::super::keysys::EUCountry::Finland => {
                                proto::keysys::EUCountry::Finland
                            }
                            super::super::keysys::EUCountry::Greece => {
                                proto::keysys::EUCountry::Greece
                            }
                            super::super::keysys::EUCountry::Germany => {
                                proto::keysys::EUCountry::Germany
                            }
                            super::super::keysys::EUCountry::Hungary => {
                                proto::keysys::EUCountry::Hungary
                            }
                            super::super::keysys::EUCountry::Italy => {
                                proto::keysys::EUCountry::Italy
                            }
                            super::super::keysys::EUCountry::Ireland => {
                                proto::keysys::EUCountry::Ireland
                            }
                            super::super::keysys::EUCountry::Latvia => {
                                proto::keysys::EUCountry::Latvia
                            }
                            super::super::keysys::EUCountry::Luxembourg => {
                                proto::keysys::EUCountry::Luxembourg
                            }
                            super::super::keysys::EUCountry::Lithuania => {
                                proto::keysys::EUCountry::Lithuania
                            }
                            super::super::keysys::EUCountry::Liechtenstein => {
                                proto::keysys::EUCountry::Liechtenstein
                            }
                            super::super::keysys::EUCountry::Malta => {
                                proto::keysys::EUCountry::Malta
                            }
                            super::super::keysys::EUCountry::Netherlands => {
                                proto::keysys::EUCountry::Netherlands
                            }
                            super::super::keysys::EUCountry::Portugal => {
                                proto::keysys::EUCountry::Portugal
                            }
                            super::super::keysys::EUCountry::Poland => {
                                proto::keysys::EUCountry::Poland
                            }
                            super::super::keysys::EUCountry::Romania => {
                                proto::keysys::EUCountry::Romania
                            }
                            super::super::keysys::EUCountry::Slovenia => {
                                proto::keysys::EUCountry::Slovenia
                            }
                            super::super::keysys::EUCountry::Slovakia => {
                                proto::keysys::EUCountry::Slovakia
                            }
                            super::super::keysys::EUCountry::Spain => {
                                proto::keysys::EUCountry::Spain
                            }
                            super::super::keysys::EUCountry::Sweden => {
                                proto::keysys::EUCountry::Sweden
                            }
                        });
                }
                Some(super::super::keysys::DomainUpdateTLD::DE(d)) => {
                    e.de_abuse_contact = d.abuse_contact.as_ref().cloned();
                    e.de_accept_trustee_tac = d.accept_trustee_tac.as_ref().map(|t| match t {
                        super::super::keysys::DETrustee::None => proto::keysys::DETrustee::None,
                        super::super::keysys::DETrustee::Annually => {
                            proto::keysys::DETrustee::Annual
                        }
                        super::super::keysys::DETrustee::Monthly => {
                            proto::keysys::DETrustee::Monthly
                        }
                    });
                    e.de_general_request = d.general_request.as_ref().cloned();
                    e.de_holder_person = d.holder_person;
                }
                Some(super::super::keysys::DomainUpdateTLD::FR(d)) => {
                    e.fr_accept_trustee_tac = d.accept_trustee_tac;
                }
                Some(super::super::keysys::DomainUpdateTLD::Name(d)) => {
                    e.name_emailforward = d.email_forward.as_ref().cloned();
                }
                Some(super::super::keysys::DomainUpdateTLD::RS(d)) => {
                    match &d.owner {
                        Some(super::super::keysys::RsId::IDCard(n)) => {
                            e.rs_owner_idcard = Some(n.clone())
                        }
                        Some(super::super::keysys::RsId::CompanyNumber(n)) => {
                            e.rs_owner_company_number = Some(n.clone())
                        }
                        None => {}
                    }
                    match &d.tech {
                        Some(super::super::keysys::RsId::IDCard(n)) => {
                            e.rs_tech_idcard = Some(n.clone())
                        }
                        Some(super::super::keysys::RsId::CompanyNumber(n)) => {
                            e.rs_tech_company_number = Some(n.clone())
                        }
                        None => {}
                    }
                    match &d.admin {
                        Some(super::super::keysys::RsId::IDCard(n)) => {
                            e.rs_admin_idcard = Some(n.clone())
                        }
                        Some(super::super::keysys::RsId::CompanyNumber(n)) => {
                            e.rs_admin_company_number = Some(n.clone())
                        }
                        None => {}
                    }
                }
                Some(super::super::keysys::DomainUpdateTLD::US(d)) => {
                    e.us_purpose = d.purpose.as_ref().map(|p| match p {
                        super::super::keysys::USPurpose::Business => {
                            proto::keysys::USPurpose::Business
                        }
                        super::super::keysys::USPurpose::Government => {
                            proto::keysys::USPurpose::Government
                        }
                        super::super::keysys::USPurpose::Personal => {
                            proto::keysys::USPurpose::Personal
                        }
                        super::super::keysys::USPurpose::NonProfit => {
                            proto::keysys::USPurpose::NonProfit
                        }
                        super::super::keysys::USPurpose::Educational => {
                            proto::keysys::USPurpose::Educational
                        }
                    });
                    e.us_category = d.category.as_ref().map(|c| match c {
                        super::super::keysys::USCategory::Citizen => {
                            proto::keysys::USCategory::Citizen
                        }
                        super::super::keysys::USCategory::PermanentResident => {
                            proto::keysys::USCategory::PermanentResident
                        }
                        super::super::keysys::USCategory::USOrganisation => {
                            proto::keysys::USCategory::USOrganisation
                        }
                        super::super::keysys::USCategory::OfficeOrFacility => {
                            proto::keysys::USCategory::OfficeOrFacility
                        }
                        super::super::keysys::USCategory::RegularActivity => {
                            proto::keysys::USCategory::RegularActivity
                        }
                    });
                    e.us_validator = d.validator.as_ref().cloned();
                }
            }

            exts.push(proto::EPPCommandExtensionType::KeysysUpdate(
                proto::keysys::Update::Domain(e),
            ));
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(nominet_ext) = &req.nominet_ext {
        if client.nominet_domain_ext {
            exts.push(proto::EPPCommandExtensionType::NominetDomainExtUpdate(
                nominet_ext.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    let command = proto::EPPUpdate::Domain(proto::domain::EPPDomainUpdate {
        name: req.name.clone(),
        add: if adds.is_empty() {
            None
        } else {
            Some(proto::domain::EPPDomainUpdateAddRemove { params: adds })
        },
        remove: if rems.is_empty() {
            None
        } else {
            Some(proto::domain::EPPDomainUpdateAddRemove { params: rems })
        },
        change: if is_not_change {
            None
        } else {
            Some(proto::domain::EPPDomainUpdateChange {
                registrant: if no_registrant {
                    None
                } else {
                    req.new_registrant.clone()
                },
                auth_info: req
                    .new_auth_info
                    .as_ref()
                    .map(|a| proto::domain::EPPDomainAuthInfo {
                        password: Some(a.clone()),
                    }),
            })
        },
    });
    Ok((
        proto::EPPCommandType::Update(Box::new(command)),
        match exts.len() {
            0 => None,
            _ => Some(exts),
        },
    ))
}

pub fn handle_verisign_sync(
    client: &ServerFeatures,
    req: &VerisignSyncRequest,
) -> HandleReqReturn<UpdateResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;

    if !client.verisign_sync_supported {
        return Err(Err(Error::Unsupported));
    }

    let mut exts = vec![];
    super::verisign::handle_verisign_namestore_erratum(client, &mut exts);

    exts.push(proto::EPPCommandExtensionType::VerisignSyncUpdate(
        proto::verisign::EPPSyncUpdate {
            month_day: proto::verisign::EPPSyncUpdateMonthDay {
                month: req.month,
                day: req.day,
            },
        },
    ));

    let command = proto::EPPUpdate::Domain(proto::domain::EPPDomainUpdate {
        name: req.name.clone(),
        add: None,
        remove: None,
        change: Some(proto::domain::EPPDomainUpdateChange {
            registrant: None,
            auth_info: None,
        }),
    });
    Ok((
        proto::EPPCommandType::Update(Box::new(command)),
        match exts.len() {
            0 => None,
            _ => Some(exts),
        },
    ))
}

pub fn handle_update_response(
    response: proto::EPPResponse, _metrics: &crate::metrics::ScopedMetrics
) -> Response<UpdateResponse> {
    let fee_data = match &response.extension {
        Some(ext) => {
            let fee10 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee10UpdateData(i) => Some(i),
                _ => None,
            });
            let fee09 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee09UpdateData(i) => Some(i),
                _ => None,
            });
            let fee08 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee08UpdateData(i) => Some(i),
                _ => None,
            });
            let fee07 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee07UpdateData(i) => Some(i),
                _ => None,
            });
            let fee06 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee06UpdateData(i) => Some(i),
                _ => None,
            });
            let fee05 = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPFee05UpdateData(i) => Some(i),
                _ => None,
            });

            if let Some(f) = fee10 {
                Some(f.into())
            } else if let Some(f) = fee09 {
                Some(f.into())
            } else if let Some(f) = fee08 {
                Some(f.into())
            } else if let Some(f) = fee07 {
                Some(f.into())
            } else if let Some(f) = fee06 {
                Some(f.into())
            } else {
                fee05.map(|f| f.into())
            }
        }
        None => None,
    };

    let donuts_fee_data = match &response.extension {
        Some(ext) => {
            let charge = ext.value.iter().find_map(|p| match p {
                proto::EPPResponseExtensionType::EPPDonutsChargeUpdateData(i) => Some(i),
                _ => None,
            });

            charge.map(Into::into)
        }
        None => None,
    };

    Response::Ok(UpdateResponse {
        pending: response.is_pending(),
        fee_data,
        donuts_fee_data,
    })
}

pub fn handle_renew(client: &ServerFeatures, req: &RenewRequest) -> HandleReqReturn<RenewResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;
    let command = proto::EPPRenew::Domain(proto::domain::EPPDomainRenew {
        name: req.name.clone(),
        period: req.add_period.as_ref().map(Into::into),
        current_expiry_date: req.cur_expiry_date.date_naive(),
    });
    let mut ext = vec![];

    if let Some(fee_agreement) = &req.fee_agreement {
        if client.fee_supported {
            ext.push(proto::EPPCommandExtensionType::EPPFee10Renew(
                fee_agreement.into(),
            ));
        } else if client.fee_011_supported {
            ext.push(proto::EPPCommandExtensionType::EPPFee011Renew(
                fee_agreement.into(),
            ));
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    match &req.isnic_payment {
        Some(e) => {
            if client.isnic_domain_supported {
                ext.push(proto::EPPCommandExtensionType::ISNICDomainRenew(e.into()))
            } else {
                return Err(Err(Error::Unsupported));
            }
        }
        None => {
            if client.isnic_domain_supported {
                return Err(Err(Error::Err(
                    "payment extension required for ISNIC".to_string(),
                )));
            }
        }
    }

    if let Some(keysys) = &req.keysys {
        if client.keysys_supported {
            ext.push(proto::EPPCommandExtensionType::KeysysRenew(
                proto::keysys::Renew::Domain(proto::keysys::DomainRenew {
                    accept_premium_price: Some(keysys.accept_premium_price),
                    promotion_code: keysys.promotion_code.as_ref().cloned(),
                }),
            ));
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    super::verisign::handle_verisign_namestore_erratum(client, &mut ext);
    super::fee::handle_donuts_fee_agreement(client, &req.donuts_fee_agreement, &mut ext)?;

    Ok((
        proto::EPPCommandType::Renew(command),
        match ext.is_empty() {
            true => None,
            false => Some(ext),
        },
    ))
}

pub fn handle_renew_response(
    response: proto::EPPResponse, _metrics: &crate::metrics::ScopedMetrics
) -> Response<RenewResponse> {
    let pending = response.is_pending();
    match response.data {
        Some(value) => match value.value {
            proto::EPPResultDataValue::EPPDomainRenewResult(domain_renew) => {
                let mut res: RenewResponse = (domain_renew, &response.extension).try_into()?;
                res.pending = pending;
                Ok(res)
            }
            _ => Err(Error::ServerInternal),
        },
        None => Err(Error::ServerInternal),
    }
}

pub fn handle_transfer_query(
    client: &ServerFeatures,
    req: &TransferQueryRequest,
) -> HandleReqReturn<TransferResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    if client.isnic_contact_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;

    if let Some(auth_info) = &req.auth_info {
        if !auth_info.is_empty() {
            check_pass(auth_info)?;
        }
    }

    let command = proto::EPPTransfer {
        operation: proto::EPPTransferOperation::Query,
        command: proto::EPPTransferCommand::DomainQuery(proto::domain::EPPDomainCheck {
            name: req.name.clone(),
            auth_info: req
                .auth_info
                .as_ref()
                .map(|a| proto::domain::EPPDomainAuthInfo {
                    password: Some(a.clone()),
                }),
        }),
    };
    let mut ext = vec![];
    super::verisign::handle_verisign_namestore_erratum(client, &mut ext);
    Ok((
        proto::EPPCommandType::Transfer(command),
        match ext.is_empty() {
            true => None,
            false => Some(ext),
        },
    ))
}

pub fn handle_transfer_request(
    client: &ServerFeatures,
    req: &TransferRequestRequest,
) -> HandleReqReturn<TransferResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    if client.isnic_contact_supported {
        return Err(Err(Error::Unsupported));
    }
    check_domain(&req.name)?;

    if !req.auth_info.is_empty() {
        check_pass(&req.auth_info)?;
    }

    let command = proto::EPPTransfer {
        operation: proto::EPPTransferOperation::Request,
        command: proto::EPPTransferCommand::DomainRequest(proto::domain::EPPDomainTransfer {
            name: req.name.clone(),
            period: req.add_period.as_ref().map(|p| p.into()),
            auth_info: Some(proto::domain::EPPDomainAuthInfo {
                password: Some(req.auth_info.clone()),
            }),
        }),
    };
    let mut ext = vec![];

    if let Some(fee_agreement) = &req.fee_agreement {
        if client.fee_supported {
            ext.push(proto::EPPCommandExtensionType::EPPFee10Transfer(
                fee_agreement.into(),
            ));
        } else if client.fee_011_supported {
            ext.push(proto::EPPCommandExtensionType::EPPFee011Transfer(
                fee_agreement.into(),
            ));
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    if let Some(keysys) = &req.keysys {
        if client.keysys_supported {
            ext.push(proto::EPPCommandExtensionType::KeysysTransfer(
                proto::keysys::Transfer::Domain(proto::keysys::DomainTransfer {
                    accept_premium_price: Some(keysys.accept_premium_price),
                    accept_quarantine: Some(keysys.accept_quarantine),
                    accept_trade: Some(keysys.accept_trade),
                    allocation_token: keysys.allocation_token.as_ref().cloned(),
                    at_request_authcode: if keysys.at_request_authcode {
                        Some(true)
                    } else {
                        None
                    },
                    be_request_authcode: if keysys.be_request_authcode {
                        Some(true)
                    } else {
                        None
                    },
                    eu_accept_trustee_tac: None,
                    eu_registrant_lang: None,
                    eu_registrant_citizenship: None,
                    promotion_code: keysys.promotion_code.as_ref().cloned(),
                }),
            ));
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    super::verisign::handle_verisign_namestore_erratum(client, &mut ext);
    super::fee::handle_donuts_fee_agreement(client, &req.donuts_fee_agreement, &mut ext)?;

    if let Some(eurid_data) = &req.eurid_data {
        if client.eurid_domain_support {
            ext.push(proto::EPPCommandExtensionType::EURIDDomainTransfer(
                eurid_data.into(),
            ))
        } else {
            return Err(Err(Error::Unsupported));
        }
    }

    Ok((
        proto::EPPCommandType::Transfer(command),
        match ext.is_empty() {
            true => None,
            false => Some(ext),
        },
    ))
}

pub fn handle_transfer_cancel(
    client: &ServerFeatures,
    req: &TransferAcceptRejectRequest,
) -> HandleReqReturn<TransferResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    if client.isnic_contact_supported {
        return Err(Err(Error::Unsupported));
    }
    if client.eurid_domain_support {
        return Err(Err(Error::Unsupported));
    }

    check_domain(&req.name)?;

    if let Some(auth_info) = &req.auth_info {
        if !auth_info.is_empty() {
            check_pass(auth_info)?;
        }
    }

    let command = proto::EPPTransfer {
        operation: proto::EPPTransferOperation::Cancel,
        command: proto::EPPTransferCommand::DomainRequest(proto::domain::EPPDomainTransfer {
            name: req.name.clone(),
            period: None,
            auth_info: req
                .auth_info
                .as_ref()
                .map(|a| proto::domain::EPPDomainAuthInfo {
                    password: Some(a.clone()),
                }),
        }),
    };
    let mut ext = vec![];
    super::verisign::handle_verisign_namestore_erratum(client, &mut ext);
    Ok((
        proto::EPPCommandType::Transfer(command),
        match ext.is_empty() {
            true => None,
            false => Some(ext),
        },
    ))
}

pub fn handle_transfer_accept(
    client: &ServerFeatures,
    req: &TransferAcceptRejectRequest,
) -> HandleReqReturn<TransferResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    if client.isnic_contact_supported {
        return Err(Err(Error::Unsupported));
    }
    if client.eurid_domain_support {
        return Err(Err(Error::Unsupported));
    }

    check_domain(&req.name)?;

    if let Some(auth_info) = &req.auth_info {
        if !auth_info.is_empty() {
            check_pass(auth_info)?;
        }
    }

    let command = proto::EPPTransfer {
        operation: proto::EPPTransferOperation::Accept,
        command: proto::EPPTransferCommand::DomainRequest(proto::domain::EPPDomainTransfer {
            name: req.name.clone(),
            period: None,
            auth_info: req
                .auth_info
                .as_ref()
                .map(|a| proto::domain::EPPDomainAuthInfo {
                    password: Some(a.clone()),
                }),
        }),
    };
    let mut ext = vec![];
    super::verisign::handle_verisign_namestore_erratum(client, &mut ext);
    Ok((
        proto::EPPCommandType::Transfer(command),
        match ext.is_empty() {
            true => None,
            false => Some(ext),
        },
    ))
}

pub fn handle_transfer_reject(
    client: &ServerFeatures,
    req: &TransferAcceptRejectRequest,
) -> HandleReqReturn<TransferResponse> {
    if !client.domain_supported {
        return Err(Err(Error::Unsupported));
    }
    if client.isnic_contact_supported {
        return Err(Err(Error::Unsupported));
    }
    if client.eurid_domain_support {
        return Err(Err(Error::Unsupported));
    }

    check_domain(&req.name)?;

    if let Some(auth_info) = &req.auth_info {
        if !auth_info.is_empty() {
            check_pass(auth_info)?;
        }
    }

    let command = proto::EPPTransfer {
        operation: proto::EPPTransferOperation::Reject,
        command: proto::EPPTransferCommand::DomainRequest(proto::domain::EPPDomainTransfer {
            name: req.name.clone(),
            period: None,
            auth_info: req
                .auth_info
                .as_ref()
                .map(|a| proto::domain::EPPDomainAuthInfo {
                    password: Some(a.clone()),
                }),
        }),
    };
    let mut ext = vec![];
    super::verisign::handle_verisign_namestore_erratum(client, &mut ext);

    Ok((
        proto::EPPCommandType::Transfer(command),
        match ext.is_empty() {
            true => None,
            false => Some(ext),
        },
    ))
}

pub fn handle_transfer_response(
    response: proto::EPPResponse, _metrics: &crate::metrics::ScopedMetrics
) -> Response<TransferResponse> {
    let pending = response.is_pending();
    match response.data {
        Some(value) => match value.value {
            proto::EPPResultDataValue::EPPDomainTransferResult(domain_transfer) => {
                let mut res: TransferResponse =
                    (domain_transfer, &response.extension).try_into()?;
                res.pending = pending;
                Ok(res)
            }
            _ => Err(Error::ServerInternal),
        },
        None => Err(Error::ServerInternal),
    }
}

#[cfg(test)]
mod domain_tests {
    #[test]
    fn claims_check() {
        const XML_DATA: &str = r#"
<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<epp xmlns="urn:ietf:params:xml:ns:epp-1.0">
  <response>
    <result code="1000">
     <msg>Command completed successfully</msg>
    </result>
    <extension>
     <launch:chkData
      xmlns:launch="urn:ietf:params:xml:ns:launch-1.0">
      <launch:phase>claims</launch:phase>
      <launch:cd>
        <launch:name exists="1">domain3.example</launch:name>
        <launch:claimKey validatorID="tmch">
        2013041500/2/6/9/rJ1NrDO92vDsAzf7EQzgjX4R0000000001
        </launch:claimKey>
        <launch:claimKey validatorID="custom-tmch">
        20140423200/1/2/3/rJ1Nr2vDsAzasdff7EasdfgjX4R000000002
        </launch:claimKey>
      </launch:cd>
     </launch:chkData>
    </extension>
    <trID>
     <clTRID>ABC-12345</clTRID>
     <svTRID>54321-XYZ</svTRID>
    </trID>
  </response>
</epp>"#;
        let res: super::proto::EPPMessage = xml_serde::from_str(XML_DATA).unwrap();
        let res = match res.message {
            super::proto::EPPMessageType::Response(r) => r,
            _ => unreachable!(),
        };
        let data = super::handle_claims_check_response(*res).unwrap();
        assert!(data.exists);
        assert_eq!(data.claims_key.len(), 2);
        let claims_key_1 = data.claims_key.get(0).unwrap();
        let claims_key_2 = data.claims_key.get(1).unwrap();
        assert_eq!(claims_key_1.validator_id.as_ref().unwrap(), "tmch");
        assert_eq!(
            claims_key_1.key,
            "2013041500/2/6/9/rJ1NrDO92vDsAzf7EQzgjX4R0000000001"
        );
        assert_eq!(claims_key_2.validator_id.as_ref().unwrap(), "custom-tmch");
        assert_eq!(
            claims_key_2.key,
            "20140423200/1/2/3/rJ1Nr2vDsAzasdff7EasdfgjX4R000000002"
        );
    }

    #[test]
    fn launch_info() {
        const XML_DATA: &str = r#"
<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<epp xmlns="urn:ietf:params:xml:ns:epp-1.0">
  <response>
    <result code="1000">
      <msg>Command completed successfully</msg>
    </result>
    <resData>
      <domain:infData
       xmlns:domain="urn:ietf:params:xml:ns:domain-1.0">
        <domain:name>domain.example</domain:name>
        <domain:roid>EXAMPLE1-REP</domain:roid>
        <domain:status s="pendingCreate"/>
        <domain:registrant>jd1234</domain:registrant>
        <domain:contact type="admin">sh8013</domain:contact>
        <domain:contact type="tech">sh8013</domain:contact>
        <domain:clID>ClientX</domain:clID>
        <domain:crID>ClientY</domain:crID>
        <domain:crDate>2012-04-03T22:00:00.0Z</domain:crDate>
        <domain:authInfo>
          <domain:pw>2fooBAR</domain:pw>
        </domain:authInfo>
      </domain:infData>
    </resData>
    <extension>
      <launch:infData
       xmlns:launch="urn:ietf:params:xml:ns:launch-1.0">
        <launch:phase>sunrise</launch:phase>
          <launch:applicationID>abc123</launch:applicationID>
          <launch:status s="pendingValidation"/>
          <mark:mark
            xmlns:mark="urn:ietf:params:xml:ns:mark-1.0">
             Test
         </mark:mark>
      </launch:infData>
    </extension>
    <trID>
      <clTRID>ABC-12345</clTRID>
      <svTRID>54321-XYZ</svTRID>
    </trID>
  </response>
</epp>"#;
        let res: super::proto::EPPMessage = xml_serde::from_str(XML_DATA).unwrap();
        let res = match res.message {
            super::proto::EPPMessageType::Response(r) => r,
            _ => unreachable!(),
        };
        let data = super::handle_info_response(*res).unwrap();
        assert_eq!(data.name, "domain.example");
        let launch_info = data.launch_info.unwrap();
        assert_eq!(
            launch_info.phase.phase_type,
            super::launch::PhaseType::Sunrise
        );
        assert_eq!(launch_info.application_id.unwrap(), "abc123");
        assert_eq!(
            launch_info.status.unwrap().status_type,
            super::launch::LaunchStatusType::PendingValidation
        );
        assert_eq!(
            launch_info.mark.unwrap(),
            r#"<mark:mark xmlns="urn:ietf:params:xml:ns:epp-1.0" xmlns:launch="urn:ietf:params:xml:ns:launch-1.0" xmlns:mark="urn:ietf:params:xml:ns:mark-1.0">Test</mark:mark>"#
        );
    }

    #[test]
    fn create_info() {
        const XML_DATA: &str = r#"
<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<epp xmlns="urn:ietf:params:xml:ns:epp-1.0">
  <response>
    <result code="1001">
      <msg>Command completed successfully; action pending</msg>
    </result>
    <resData>
      <domain:creData
         xmlns:domain="urn:ietf:params:xml:ns:domain-1.0">
       <domain:name>domain.example</domain:name>
       <domain:crDate>2010-08-10T15:38:26.623854Z</domain:crDate>
      </domain:creData>
    </resData>
    <extension>
      <launch:creData
        xmlns:launch="urn:ietf:params:xml:ns:launch-1.0">
        <launch:phase>sunrise</launch:phase>
        <launch:applicationID>2393-9323-E08C-03B1
        </launch:applicationID>
      </launch:creData>
    </extension>
    <trID>
      <clTRID>ABC-12345</clTRID>
      <svTRID>54321-XYZ</svTRID>
    </trID>
  </response>
</epp>"#;
        let res: super::proto::EPPMessage = xml_serde::from_str(XML_DATA).unwrap();
        let res = match res.message {
            super::proto::EPPMessageType::Response(r) => r,
            _ => unreachable!(),
        };
        let data = super::handle_create_response(*res).unwrap();
        assert!(data.pending);
        let launch_create = data.launch_create.unwrap();
        assert_eq!(
            launch_create.phase.phase_type,
            super::launch::PhaseType::Sunrise
        );
        assert_eq!(launch_create.application_id.unwrap(), "2393-9323-E08C-03B1");
    }

    #[test]
    fn fee_check_10() {
        const XML_DATA: &str = r#"
<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<epp xmlns="urn:ietf:params:xml:ns:epp-1.0">
  <response>
    <result code="1000">
      <msg lang="en">Command completed successfully</msg>
    </result>
    <resData>
      <domain:chkData xmlns:domain="urn:ietf:params:xml:ns:domain-1.0">
        <domain:cd>
          <domain:name avail="0">test.tv</domain:name>
          <domain:reason>In use</domain:reason>
        </domain:cd>
      </domain:chkData>
    </resData>
    <extension>
      <fee:chkData xmlns:fee="urn:ietf:params:xml:ns:epp:fee-1.0">
        <fee:currency>USD</fee:currency>
        <fee:cd avail="1">
          <fee:objID>test.tv</fee:objID>
          <fee:class>STANDARD</fee:class>
          <fee:command name="transfer" standard="1" phase="open">
            <fee:period unit="y">1</fee:period>
            <fee:fee description="Transfer Fee" refundable="1" applied="delayed" grace-period="P5D">25.00</fee:fee>
          </fee:command>
        </fee:cd>
      </fee:chkData>
    </extension>
    <trID>
      <clTRID>d6b341c4-43a1-4fbd-bd24-9ada2b5c088b</clTRID>
      <svTRID>2dbd511a-a3d1-4308-b6aa-98334e810df8</svTRID>
    </trID>
  </response>
</epp>"#;
        let res: super::proto::EPPMessage = xml_serde::from_str(XML_DATA).unwrap();
        let res = match res.message {
            super::proto::EPPMessageType::Response(r) => r,
            _ => unreachable!(),
        };
        let data = super::handle_check_response(*res).unwrap();
        assert!(!data.avail);
        assert_eq!(data.reason.unwrap(), "In use");
        let fee_check = data.fee_check.unwrap();
        assert!(fee_check.available);
        let command = fee_check.commands.get(0).unwrap();
        assert_eq!(command.command, super::fee::Command::Transfer);
        assert_eq!(command.class.as_ref().unwrap(), "STANDARD");
        assert_eq!(command.currency, "USD");
        assert_eq!(command.fees.len(), 1);
    }
}
